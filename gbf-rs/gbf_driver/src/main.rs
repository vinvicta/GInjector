#![feature(backtrace_frames)]

use clap::{Parser, ValueEnum};
use gbf_core::{
    cfg_dot::{CfgDotConfig, DotRenderableGraph},
    decompiler::{
        ast::visitors::emit_context::{
            EmitContext, EmitContextBuilder, EmitVerbosity, IndentStyle,
        },
        function_decompiler::FunctionDecompilerBuilder,
    },
    function::Function,
    module::{Module, ModuleBuilder},
};
use log::{error, info};
use rayon::prelude::*;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// GBF decompilation driver
#[derive(Parser, Debug)]
#[clap(
    name = "gbf_driver",
    version,
    about = "Driver for decompiling GBF modules and functions"
)]
struct Cli {
    /// Input file or directory containing GBF bytecode
    #[clap(value_parser)]
    input: PathBuf,

    /// Optional output directory for decompiled output (if not provided, prints to stdout)
    #[clap(long, value_parser)]
    output: Option<PathBuf>,

    /// Decompilation mode: decompile an entire module or one or all functions.
    #[clap(short, long, value_enum, default_value_t = Mode::Module)]
    mode: Mode,

    /// If mode is 'function', the name of the function to decompile.
    /// If omitted, decompiles all functions concurrently.
    #[clap(short, long)]
    function: Option<String>,

    /// Minify the decompiled output (instead of pretty printing)
    #[clap(long)]
    minified: bool,

    /// Do not include SSA versions (by default SSA versions are included)
    #[clap(long)]
    no_ssa: bool,

    /// Indent style: Allman or KAndR (default: Allman)
    #[clap(long, value_enum, default_value_t = Indent::Allman)]
    indent: Indent,

    /// Do not format numbers as hexadecimal (by default hex formatting is enabled)
    #[clap(long)]
    no_hex: bool,

    /// Output Graphviz CFG dot files for each function in the module
    #[clap(long)]
    graphviz: bool,

    /// Directory where Graphviz dot files will be written (if not provided, prints to stdout)
    #[clap(long, value_parser)]
    graphviz_output: Option<PathBuf>,
}

/// Decompilation mode: either an entire module or function(s).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Mode {
    Module,
    Function,
}

/// Indent style for decompiled output.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Indent {
    Allman,
    KAndR,
}

fn main() {
    // Initialize log4rs using the logging_config.yaml file in the crate root.
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("logging_config.yaml");
    if let Err(e) = log4rs::init_file(config_path, Default::default()) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    let args = Cli::parse();

    // Build the EmitContext based on command-line options.
    let emit_context: EmitContext = EmitContextBuilder::default()
        .verbosity(if args.minified {
            EmitVerbosity::Minified
        } else {
            EmitVerbosity::Pretty
        })
        .include_ssa_versions(!args.no_ssa)
        .indent_style(match args.indent {
            Indent::Allman => IndentStyle::Allman,
            Indent::KAndR => IndentStyle::KAndR,
        })
        .format_number_hex(!args.no_hex)
        .build();

    // If the input is a directory, process each file in parallel.
    if args.input.is_dir() {
        fs::read_dir(&args.input)
            .unwrap_or_else(|e| {
                error!("Failed to read directory {}: {}", args.input.display(), e);
                std::process::exit(1);
            })
            .par_bridge()
            .for_each(|entry| {
                let entry = match entry {
                    Ok(ent) => ent,
                    Err(e) => {
                        error!("Failed to get directory entry: {}", e);
                        return;
                    }
                };

                let path = entry.path();
                if path.is_file() {
                    process_file(&path, &args, &emit_context);
                }
            });
    } else if args.input.is_file() {
        process_file(&args.input, &args, &emit_context);
    } else {
        error!(
            "Input path {} is not a valid file or directory",
            args.input.display()
        );
        std::process::exit(1);
    }
}

/// Process a single file (module) using the given options and context.
fn process_file(path: &Path, args: &Cli, context: &EmitContext) {
    info!("Processing file: {}", path.display());

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open file {}: {}", path.display(), e);
            return;
        }
    };

    let module = match ModuleBuilder::new()
        .name(
            path.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown_module".to_string()),
        )
        .reader(Box::new(file) as Box<dyn Read>)
        .build()
    {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to build module from {}: {:?}", path.display(), e);
            return;
        }
    };

    match args.mode {
        Mode::Module => {
            // Decompile the entire module.
            match module.decompile(*context) {
                Ok(decompiled) => {
                    info!("--- Decompiled module from {} ---", path.display());
                    if let Some(ref output_dir) = args.output {
                        let base = path
                            .file_stem()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_else(|| "module".into());
                        let output_path = output_dir.join(format!("{}.gs2", base));
                        if let Err(e) = fs::write(&output_path, &decompiled) {
                            error!(
                                "Failed to write decompiled module to {}: {}",
                                output_path.display(),
                                e
                            );
                        } else {
                            info!("Decompiled module written to {}", output_path.display());
                        }
                    } else {
                        println!("{}", decompiled);
                    }
                    if args.graphviz {
                        output_module_graphviz(&module, path, args);
                    }
                }
                Err(e) => {
                    error!(
                        "Module decompilation failed for {}: {:?}",
                        path.display(),
                        e
                    );
                }
            }
        }
        Mode::Function => {
            if let Some(func_name) = &args.function {
                // Decompile a single, specified function.
                let func = match module.get_function_by_name(func_name).ok() {
                    Some(f) => f,
                    None => {
                        error!(
                            "Function '{}' not found in module {}",
                            func_name,
                            path.display()
                        );
                        return;
                    }
                };
                decompile_and_output_function(func, &module, path, args, context);
            } else {
                // Decompile all functions concurrently.
                let funcs: Vec<_> = module.iter().collect();
                funcs.par_iter().for_each(|func| {
                    let func_name = func.id.name.clone().unwrap_or_else(|| "entry".to_string());
                    let func_insts_count = func.iter().map(|bb| bb.len()).sum::<usize>();
                    info!(
                        "Decompiling function {} in module {} ({} instructions)",
                        func_name,
                        module.name.clone().unwrap_or_else(|| "unknown".to_string()),
                        func_insts_count
                    );
                    // Create a function-specific EmitContext (customize as needed).
                    let func_context = EmitContextBuilder::default()
                        .include_ssa_versions(true)
                        .build();
                    let mut decompiler = FunctionDecompilerBuilder::new(func)
                        .structure_analysis_max_iterations(1000)
                        .structure_debug_mode(false)
                        .build();
                    let res = decompiler.decompile(func_context);
                    match res {
                        Ok(result) => {
                            info!("Decompiled function {}", func_name);
                            if let Some(ref output_dir) = args.output {
                                let output_path = output_dir.join(format!(
                                    "{}_{}.gs2",
                                    module.name.clone().unwrap_or_else(|| "unknown".to_string()),
                                    func_name
                                ));
                                if let Err(e) = fs::write(&output_path, &result) {
                                    error!(
                                        "Failed to write decompiled function to {}: {}",
                                        output_path.display(),
                                        e
                                    );
                                } else {
                                    info!(
                                        "Decompiled function written to {}",
                                        output_path.display()
                                    );
                                }
                            } else {
                                println!("--- Decompiled function '{}' ---\n{}", func_name, result);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Error decompiling function {} in module {} ({} instructions): {}",
                                func_name,
                                module.name.clone().unwrap_or_else(|| "unknown".to_string()),
                                func_insts_count,
                                e
                            );
                        }
                    }
                    if args.graphviz {
                        output_function_graphviz(func, path, args);
                    }
                });
            }
        }
    }
}

/// Helper to decompile a single function and output its result.
fn decompile_and_output_function(
    func: &Function,
    module: &Module,
    path: &Path,
    args: &Cli,
    context: &EmitContext,
) {
    let func_name = func.id.name.clone().unwrap_or_else(|| "entry".to_string());
    let mut decompiler = FunctionDecompilerBuilder::new(func).build();
    match decompiler.decompile(*context) {
        Ok(result) => {
            info!(
                "--- Decompiled function '{}' from {} ---",
                func_name,
                path.display()
            );
            if let Some(ref output_dir) = args.output {
                let output_path = output_dir.join(format!(
                    "{}_{}.gs2",
                    module.name.clone().unwrap_or_else(|| "unknown".to_string()),
                    func_name
                ));
                if let Err(e) = fs::write(&output_path, &result) {
                    error!(
                        "Failed to write decompiled function to {}: {}",
                        output_path.display(),
                        e
                    );
                } else {
                    info!("Decompiled function written to {}", output_path.display());
                }
            } else {
                println!("{}", result);
            }
        }
        Err(e) => {
            error!(
                "Function decompilation failed for {} in {}: {:?}",
                func_name,
                path.display(),
                e
            );
        }
    }
}

/// Output Graphviz CFG dot files for every function in the module.
fn output_module_graphviz(module: &Module, input_path: &Path, args: &Cli) {
    if let Some(ref output_dir) = args.graphviz_output {
        // Write each function's dot file to the specified directory.
        for func in module.iter() {
            let dot = func.render_dot(CfgDotConfig::default());
            let func_name = func.id.name.clone().unwrap_or_else(|| "entry".to_string());
            let filename = format!(
                "{}_{}_cfg.dot",
                module.name.clone().unwrap_or_else(|| "unknown".to_string()),
                func_name
            );
            let dot_path = output_dir.join(filename);
            if let Err(e) = fs::write(&dot_path, &dot) {
                error!(
                    "Failed to write Graphviz CFG to {}: {}",
                    dot_path.display(),
                    e
                );
            } else {
                info!("Graphviz CFG written to {}", dot_path.display());
            }
        }
    } else {
        // Print dot files to stdout if no output directory is specified.
        for func in module.iter() {
            let dot = func.render_dot(CfgDotConfig::default());
            let func_name = func.id.name.clone().unwrap_or_else(|| "entry".to_string());
            println!(
                "\n--- Graphviz CFG for function '{}' in module '{}' ---\n{}\n",
                func_name,
                input_path.display(),
                dot
            );
        }
    }
}

/// Output a Graphviz CFG dot file for a single function.
fn output_function_graphviz(func: &Function, input_path: &Path, args: &Cli) {
    if let Some(ref output_dir) = args.graphviz_output {
        let dot = func.render_dot(CfgDotConfig::default());
        let base = input_path
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| "module".into());
        let func_name = func.id.name.clone().unwrap_or_else(|| "entry".to_string());
        let filename = format!("{}_{}_cfg.dot", base, func_name);
        let dot_path = output_dir.join(filename);
        if let Err(e) = fs::write(&dot_path, &dot) {
            error!(
                "Failed to write Graphviz CFG to {}: {}",
                dot_path.display(),
                e
            );
        } else {
            info!("Graphviz CFG written to {}", dot_path.display());
        }
    } else {
        let dot = func.render_dot(CfgDotConfig::default());
        let func_name = func.id.name.clone().unwrap_or_else(|| "entry".to_string());
        println!(
            "\n--- Graphviz CFG for function '{}' in module '{}' ---\n{}\n",
            func_name,
            input_path.display(),
            dot
        );
    }
}
