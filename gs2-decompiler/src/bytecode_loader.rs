#![deny(missing_docs)]

use crate::{
    graal_io::{GraalIoError, GraalReader},
    instruction::Instruction,
    opcode::{Opcode, OpcodeError},
    operand::{Operand, OperandError},
    utils::Gs2BytecodeAddress,
};

use std::{
    collections::{BTreeSet, HashMap},
    io::Read,
};

use log::warn;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

/// Error type for bytecode operations.
#[derive(Debug, Clone, Serialize)]
pub enum BytecodeLoaderError {
    /// Error for when an invalid section type is encountered.
    InvalidSectionType(u32),

    /// Error for when an invalid section length is encountered.
    InvalidSectionLength(SectionType, u32),

    /// Error when string index is out of bounds.
    StringIndexOutOfBounds(usize, usize),

    /// Error for when there is no previous instruction when setting an operand.
    NoPreviousInstruction,

    /// Unreachable block error.
    UnreachableBlock(Gs2BytecodeAddress),

    /// Error for when an I/O error occurs.
    GraalIo(String),

    /// Error for when an invalid opcode is encountered.
    OpcodeError(String),

    /// Error for when an invalid operand is encountered.
    InvalidOperand(String),
}

impl std::fmt::Display for BytecodeLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BytecodeLoaderError::InvalidSectionType(t) => write!(f, "Invalid section type: {}", t),
            BytecodeLoaderError::InvalidSectionLength(st, l) => {
                write!(f, "Invalid section length for {}: {}", st, l)
            }
            BytecodeLoaderError::StringIndexOutOfBounds(i, l) => {
                write!(f, "String index {} is out of bounds. Length: {}", i, l)
            }
            BytecodeLoaderError::NoPreviousInstruction => {
                write!(f, "No previous instruction to set operand")
            }
            BytecodeLoaderError::UnreachableBlock(a) => write!(f, "Block at address {} is unreachable", a),
            BytecodeLoaderError::GraalIo(e) => write!(f, "GraalIo error: {}", e),
            BytecodeLoaderError::OpcodeError(e) => write!(f, "Invalid opcode: {}", e),
            BytecodeLoaderError::InvalidOperand(e) => write!(f, "Invalid operand: {}", e),
        }
    }
}

impl std::error::Error for BytecodeLoaderError {}

impl From<GraalIoError> for BytecodeLoaderError {
    fn from(err: GraalIoError) -> Self {
        BytecodeLoaderError::GraalIo(err.to_string())
    }
}

impl From<OpcodeError> for BytecodeLoaderError {
    fn from(err: OpcodeError) -> Self {
        BytecodeLoaderError::OpcodeError(err.to_string())
    }
}

impl From<OperandError> for BytecodeLoaderError {
    fn from(err: OperandError) -> Self {
        BytecodeLoaderError::InvalidOperand(err.to_string())
    }
}

impl std::fmt::Display for SectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SectionType::Gs1Flags => write!(f, "Gs1Flags"),
            SectionType::Functions => write!(f, "Functions"),
            SectionType::Strings => write!(f, "Strings"),
            SectionType::Instructions => write!(f, "Instructions"),
        }
    }
}

/// Represents a section type in a module.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum SectionType {
    /// The section contains flags for the module.
    Gs1Flags = 1,

    /// The section contains the module's functions.
    Functions = 2,

    /// The section contains the module's strings.
    Strings = 3,

    /// The section contains the module's instructions.
    Instructions = 4,
}

/// A builder for a BytecodeLoader.
pub struct BytecodeLoaderBuilder<R> {
    reader: R,
}

impl<R: std::io::Read> BytecodeLoaderBuilder<R> {
    /// Creates a new BytecodeLoaderBuilder.
    ///
    /// # Arguments
    /// - `reader`: The reader to read the bytecode from.
    ///
    /// # Returns
    /// - A new `BytecodeLoaderBuilder` instance.
    ///
    /// Example
    /// ```
    /// use gs2_decompiler::bytecode_loader::BytecodeLoaderBuilder;
    ///
    /// let reader = std::io::Cursor::new(vec![0x00, 0x00, 0x00, 0x00]);
    /// let builder = BytecodeLoaderBuilder::new(reader);
    /// ```
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Builds a `BytecodeLoader` from the builder.
    ///
    /// # Returns
    /// - A `Result` containing the `BytecodeLoader` if successful.
    ///
    /// # Errors
    /// - `BytecodeLoaderError::InvalidSectionType` if an invalid section type is encountered.
    /// - `BytecodeLoaderError::InvalidSectionLength` if an invalid section length is encountered.
    /// - `BytecodeLoaderError::StringIndexOutOfBounds` if a string index is out of bounds.
    /// - `BytecodeLoaderError::NoPreviousInstruction` if there is no previous instruction when setting an operand.
    /// - `BytecodeLoaderError::GraalIo` if an I/O error occurs.
    /// - `BytecodeLoaderError::OpcodeError` if an invalid opcode is encountered.
    pub fn build(self) -> Result<BytecodeLoader<R>, BytecodeLoaderError> {
        let mut loader = BytecodeLoader {
            block_breaks: BTreeSet::new(),
            reader: GraalReader::new(self.reader),
            function_map: HashMap::new(),
            strings: Vec::new(),
            instructions: Vec::new(),
            raw_block_graph: DiGraph::new(),
            raw_block_address_to_node: HashMap::new(),
            block_address_to_function: HashMap::new(),
        };
        loader.load()?; // Load data during construction
        Ok(loader)
    }
}

/// A structure for loading bytecode from a reader.
pub struct BytecodeLoader<R: Read> {
    reader: GraalReader<R>,
    strings: Vec<String>,

    /// A map of function names to their addresses.
    pub function_map: HashMap<Option<String>, Gs2BytecodeAddress>,

    /// The instructions in the module.
    pub instructions: Vec<Instruction>,

    // A HashSet of where block breaks occur.
    block_breaks: BTreeSet<Gs2BytecodeAddress>,

    /// The relationship between each block start address and the next block start address.
    raw_block_graph: DiGraph<Gs2BytecodeAddress, ()>,

    /// A map of block start addresses to their corresponding node in the graph.
    raw_block_address_to_node: HashMap<Gs2BytecodeAddress, NodeIndex>,

    /// A map of block start addresses to their corresponding function name.
    pub block_address_to_function: HashMap<Gs2BytecodeAddress, Option<String>>,
}

impl<R: Read> BytecodeLoader<R> {
    /// Asserts that the section length is correct.
    ///
    /// # Arguments
    /// - `section_type`: The type of the section.
    /// - `expected_length`: The expected length of the section.
    /// - `got_length`: The actual length of the section.
    ///
    /// # Returns
    /// - A `Result` indicating success or failure.
    ///
    /// # Errors
    /// - `BytecodeLoaderError::InvalidSectionLength` if the section length is incorrect.
    fn expect_section_length(
        section_type: SectionType,
        expected_length: u32,
        got_length: u32,
    ) -> Result<(), BytecodeLoaderError> {
        if expected_length != got_length {
            return Err(BytecodeLoaderError::InvalidSectionLength(
                section_type,
                got_length,
            ));
        }
        Ok(())
    }

    /// Reads the flags section from the reader.
    ///
    /// We don't actually need to do anything with the flags section, so we just read it and ignore it.
    fn read_gs1_flags(&mut self) -> Result<(), BytecodeLoaderError> {
        let section_length = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;
        let _flags = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;

        // assert that the section length is correct
        Self::expect_section_length(SectionType::Gs1Flags, 4, section_length)?;

        Ok(())
    }

    /// Insert a block start into the graph
    ///
    /// # Arguments
    /// - `address`: The address of the block.
    fn insert_block_start(&mut self, address: Gs2BytecodeAddress) {
        self.block_breaks.insert(address);
    }

    /// Reads the functions section from the reader. This section contains the names of the functions
    /// in the module.
    ///
    /// # Returns
    /// - A `Result` indicating success or failure.
    ///
    /// # Errors
    /// - `BytecodeLoaderError::InvalidSectionLength` if the section length is incorrect.
    /// - `BytecodeLoaderError::GraalIo` if an I/O error occurs.
    fn read_functions(&mut self) -> Result<(), BytecodeLoaderError> {
        let section_length = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;

        // Insert the entry point function
        self.function_map.insert(None, 0);

        // For each function, use self.reader.read_u32() to get the location of the function,
        // and then use self.reader.read_string() to get the name of the function. We should
        // only read up to section_length bytes.
        let mut bytes_read = 0;
        while bytes_read < section_length {
            let function_location =
                self.reader.read_u32().map_err(BytecodeLoaderError::from)? as Gs2BytecodeAddress;
            let function_name = self
                .reader
                .read_string()
                .map_err(BytecodeLoaderError::from)?
                .0;
            self.function_map
                .insert(Some(function_name.clone()), function_location);
            bytes_read += 4 + function_name.len() as u32;
            bytes_read += 1; // Null terminator

            self.insert_block_start(function_location);
        }

        // assert that the section length is correct
        Self::expect_section_length(SectionType::Functions, section_length, bytes_read)?;

        Ok(())
    }

    /// Reads the strings section from the reader. This section contains the strings used in the module.
    ///
    /// # Returns
    /// - A `Result` indicating success or failure.
    ///
    /// # Errors
    /// - `BytecodeLoaderError::GraalIo` if an I/O error occurs.
    /// - `BytecodeLoaderError::InvalidSectionLength` if the section length is incorrect.
    fn read_strings(&mut self) -> Result<(), BytecodeLoaderError> {
        let section_length = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;

        // For each string, use self.reader.read_string() to get the string. We should only read up to section_length bytes.
        let mut bytes_read = 0;
        while bytes_read < section_length {
            let (string, raw_len) = self
                .reader
                .read_string()
                .map_err(BytecodeLoaderError::from)?;
            self.strings.push(string.clone());
            bytes_read += raw_len;
            bytes_read += 1; // for the null terminator
        }

        // assert that the section length is correct
        Self::expect_section_length(SectionType::Strings, section_length, bytes_read)?;

        Ok(())
    }

    /// Read one opcode from the reader and return it.
    fn read_opcode(&mut self) -> Result<Opcode, BytecodeLoaderError> {
        let opcode_byte = self.reader.read_u8().map_err(BytecodeLoaderError::from)?;
        let opcode = Opcode::from_byte(opcode_byte)?;
        Ok(opcode)
    }

    /// Read one operand from the reader and return it along with the number of bytes read.
    fn read_operand(
        &mut self,
        opcode: Opcode,
    ) -> Result<Option<(Operand, usize)>, BytecodeLoaderError> {
        match opcode {
            Opcode::ImmStringByte => {
                let string_index = self.reader.read_u8().map_err(BytecodeLoaderError::from)?;
                let string = self.strings.get(string_index as usize).ok_or(
                    BytecodeLoaderError::StringIndexOutOfBounds(
                        string_index as usize,
                        self.strings.len(),
                    ),
                )?;
                Ok(Some((Operand::new_string(string), 1)))
            }
            Opcode::ImmStringShort => {
                let string_index = self.reader.read_u16().map_err(BytecodeLoaderError::from)?;
                let string = self.strings.get(string_index as usize).ok_or(
                    BytecodeLoaderError::StringIndexOutOfBounds(
                        string_index as usize,
                        self.strings.len(),
                    ),
                )?;
                Ok(Some((Operand::new_string(string), 2)))
            }
            Opcode::ImmStringInt => {
                let string_index = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;
                let string = self.strings.get(string_index as usize).ok_or(
                    BytecodeLoaderError::StringIndexOutOfBounds(
                        string_index as usize,
                        self.strings.len(),
                    ),
                )?;
                Ok(Some((Operand::new_string(string), 4)))
            }
            Opcode::ImmByte => {
                let value = self.reader.read_u8().map_err(BytecodeLoaderError::from)?;
                Ok(Some((Operand::new_number(value as i8 as i32), 1)))
            }
            Opcode::ImmShort => {
                let value = self.reader.read_u16().map_err(BytecodeLoaderError::from)?;
                Ok(Some((Operand::new_number(value as i16 as i32), 2)))
            }
            Opcode::ImmInt => {
                let value = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;
                Ok(Some((Operand::new_number(value as i32), 4)))
            }
            Opcode::ImmFloat => {
                let value = self
                    .reader
                    .read_string()
                    .map_err(BytecodeLoaderError::from)?
                    .0;
                Ok(Some((Operand::new_float(value.clone()), value.len() + 1)))
            }
            _ => Ok(None),
        }
    }

    /// Reads the instructions section from the reader. This section contains the bytecode instructions.
    fn read_instructions(&mut self) -> Result<(), BytecodeLoaderError> {
        // Add the first block start address
        self.insert_block_start(0);

        let section_length = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;

        let mut bytes_read = 0;
        while bytes_read < section_length {
            let opcode = self.read_opcode()?;
            bytes_read += 1;

            let operand = self.read_operand(opcode)?;

            if let Some(operand) = operand {
                // Separate scope for mutable borrow of instructions
                {
                    let last_instruction = self
                        .instructions
                        .last_mut()
                        .ok_or(BytecodeLoaderError::NoPreviousInstruction)?;

                    last_instruction.set_operand(operand.0.clone());
                }

                bytes_read += operand.1 as u32;

                assert!(self.instructions.last().is_some());

                // We can unwrap here because we know that the last instruction exists in the scope above
                if self.instructions.last().unwrap().opcode.has_jump_target() {
                    self.insert_block_start(operand.0.get_number_value()? as Gs2BytecodeAddress);
                }
            } else {
                // Create a new instruction
                let address = self.instructions.len();
                self.instructions.push(Instruction::new(opcode, address));

                if opcode.is_block_end() {
                    let current_address = address as Gs2BytecodeAddress;
                    self.insert_block_start(current_address + 1);
                }
            }
        }

        // Verify the section length
        Self::expect_section_length(SectionType::Instructions, section_length, bytes_read)?;

        // Handle the case of empty instructions
        if self.instructions.is_empty() {
            warn!("No instructions were loaded.");
            self.block_breaks.clear();
        }

        // Validate all addresses
        let instruction_count = self.instructions.len() as Gs2BytecodeAddress;
        for address in self.block_breaks.iter() {
            // It is legal to jump to the "end" of the instructions, but not past it.
            if *address > instruction_count {
                return Err(BytecodeLoaderError::InvalidOperand(
                    "Jump target exceeds instruction count".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Loads the bytecode from the reader into the structure.
    ///
    /// # Returns
    /// - A `Result` indicating success or failure.
    ///
    /// # Errors
    /// - `BytecodeLoaderError::InvalidSectionType` if an invalid section type is encountered.
    /// - `BytecodeLoaderError::InvalidSectionLength` if an invalid section length is encountered.
    /// - `BytecodeLoaderError::StringIndexOutOfBounds` if a string index is out of bounds.
    /// - `BytecodeLoaderError::NoPreviousInstruction` if there is no previous instruction when setting an operand.
    /// - `BytecodeLoaderError::GraalIo` if an I/O error occurs.
    /// - `BytecodeLoaderError::OpcodeError` if an invalid opcode is encountered.
    /// - `BytecodeLoaderError::InvalidOperand` if an invalid operand is encountered.
    fn load(&mut self) -> Result<(), BytecodeLoaderError> {
        // TODO: I know there will only be 4 sections, but I'd like to make this more dynamic.
        for _ in 0..4 {
            let section_type = self.read_section_type()?;
            match section_type {
                SectionType::Gs1Flags => {
                    self.read_gs1_flags()?;
                }
                SectionType::Functions => {
                    self.read_functions()?;
                }
                SectionType::Strings => {
                    self.read_strings()?;
                }
                SectionType::Instructions => {
                    self.read_instructions()?;
                }
            }
        }

        // After reading in all of the block breaks, we can now create the graph.
        for block_break in self.block_breaks.iter() {
            let node = self.raw_block_graph.add_node(*block_break);
            self.raw_block_address_to_node.insert(*block_break, node);
        }

        // Iterate through each instruction to figure out the edges
        for instruction in self.instructions.iter() {
            let current_instruction_address = instruction.address as Gs2BytecodeAddress;
            let current_block_address = self.find_block_start_address(current_instruction_address);
            // if the instruction is the last instruction in the block
            let is_block_end = self
                .block_breaks
                .contains(&(current_instruction_address + 1));

            // If the current instruction is a jump, then we need to add an edge to the target block start
            if instruction.opcode.has_jump_target() {
                let source_node = self
                    .raw_block_address_to_node
                    .get(&current_block_address)
                    // We can unwrap here because we know that the current block address exists
                    // If it doesn't, then there is a bug that needs to be fixed in the internal
                    // logic of the loader.
                    .unwrap();

                // Unwrap here because we know that the operand exists due to a previous check in
                // `read_instructions`
                let target_address =
                    instruction.operand.as_ref().unwrap().get_number_value()? as Gs2BytecodeAddress;

                // Also unwrap here because we know that the target address exists in the block breaks
                let target_node = self.raw_block_address_to_node.get(&target_address).unwrap();

                self.raw_block_graph
                    .add_edge(*source_node, *target_node, ());
            }

            // If the current opcode has a fallthrough, then we need to add an edge to the next block start
            if is_block_end && instruction.opcode.connects_to_next_block() {
                let source_node = self
                    .raw_block_address_to_node
                    .get(&current_block_address)
                    // We can unwrap here because we know that the current block address exists
                    // If it doesn't, then there is a bug that needs to be fixed in the internal
                    // logic of the loader.
                    .unwrap();

                // Find the next block start address
                let next_block_address = current_instruction_address + 1;

                // Also unwrap here because we know that the target address exists in the block breaks
                let target_node = self
                    .raw_block_address_to_node
                    .get(&next_block_address)
                    .unwrap();

                self.raw_block_graph
                    .add_edge(*source_node, *target_node, ());
            }
        }

        // Iterate through each function
        for (function_name, function_address) in self.function_map.iter() {
            assert_eq!(
                self.raw_block_graph.node_count(),
                self.raw_block_address_to_node.len(),
                "Graph node count and block address map size do not match!"
            );
            for (&block_address, &node) in &self.raw_block_address_to_node {
                assert!(
                    self.raw_block_graph.node_indices().any(|n| n == node),
                    "Node {:?} for block address {} is missing in the graph.",
                    node,
                    block_address
                );
            }

            if let Some(function_node) = self.raw_block_address_to_node.get(function_address) {
                let mut dfs = petgraph::visit::Dfs::new(&self.raw_block_graph, *function_node);

                while let Some(node) = dfs.next(&self.raw_block_graph) {
                    // Map node back to block address.
                    if let Some(block_address) = self
                        .raw_block_address_to_node
                        .iter()
                        .find_map(|(k, v)| if *v == node { Some(*k) } else { None })
                    {
                        self.block_address_to_function
                            .insert(block_address, function_name.clone());
                    } else {
                        warn!("Node {:?} has no matching block address.", node);
                    }
                }
            } else {
                warn!(
                    "Function '{:?}' at address {} has no corresponding node in raw_block_address_to_node.",
                    function_name, function_address
                );
            }
        }

        Ok(())
    }

    /// Get the function name for a given address.
    ///
    /// # Arguments
    /// - `address`: The address to get the function name for.
    ///
    /// # Returns
    /// - The function name, if it exists.
    ///
    /// # Errors
    /// - `BytecodeLoaderError::UnreachableBlock` if the block is unreachable, and therefore,
    ///   the function name cannot be determined.
    pub fn get_function_name_for_address(
        &self,
        address: Gs2BytecodeAddress,
    ) -> Result<Option<String>, BytecodeLoaderError> {
        let block_start = self.find_block_start_address(address);
        Ok(self
            .block_address_to_function
            .get(&block_start)
            // return error if the block is unreachable
            .ok_or(BytecodeLoaderError::UnreachableBlock(block_start))?
            .clone())
    }

    /// Checks if an instruction is reachable.
    ///
    /// # Arguments
    /// - `address`: The address to check.
    ///
    /// # Returns
    /// - `true` if the instruction is reachable, `false` otherwise.
    pub fn is_instruction_reachable(&self, address: Gs2BytecodeAddress) -> bool {
        let blk = self.find_block_start_address(address);
        self.block_address_to_function.contains_key(&blk)
    }

    /// Helper function to figure out what block the address is in. This basically looks
    /// at the argument, and finds the closest block start address that is less than or equal
    ///
    /// # Arguments
    /// - `address`: The address to find the block for.
    ///
    /// # Returns
    /// - The block start address.
    pub fn find_block_start_address(&self, address: Gs2BytecodeAddress) -> Gs2BytecodeAddress {
        let mut block_start = 0;
        for block_break in self.block_breaks.iter() {
            if *block_break > address {
                break;
            }
            block_start = *block_break;
        }
        block_start
    }

    /// Reads a section type from the reader.
    fn read_section_type(&mut self) -> Result<SectionType, BytecodeLoaderError> {
        let section_type = self.reader.read_u32().map_err(BytecodeLoaderError::from)?;
        match section_type {
            1 => Ok(SectionType::Gs1Flags),
            2 => Ok(SectionType::Functions),
            3 => Ok(SectionType::Strings),
            4 => Ok(SectionType::Instructions),
            _ => Err(BytecodeLoaderError::InvalidSectionType(section_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{bytecode_loader::BytecodeLoaderBuilder, utils::Gs2BytecodeAddress};

    #[test]
    fn test_load() {
        let reader = std::io::Cursor::new(vec![
            0x00, 0x00, 0x00, 0x01, // Section type: Gs1Flags
            0x00, 0x00, 0x00, 0x04, // Length: 4
            0x00, 0x00, 0x00, 0x00, // Flags: 0
            0x00, 0x00, 0x00, 0x02, // Section type: Functions
            0x00, 0x00, 0x00, 0x09, // Length: 9
            0x00, 0x00, 0x00, 0x00, // Function location: 0
            0x6d, 0x61, 0x69, 0x6e, // Function name: "main"
            0x00, // Null terminator
            0x00, 0x00, 0x00, 0x03, // Section type: Strings
            0x00, 0x00, 0x00, 0x04, // Length: 4
            0x61, 0x62, 0x63, 0x00, // String: "abc"
            0x00, 0x00, 0x00, 0x04, // Section type: Instructions
            0x00, 0x00, 0x00, 0x0c, // Length: 12
            0x01, // Opcode: Jmp
            0xF3, // Opcode: ImmByte
            0x01, // Operand: 1
            0x14, // Opcode: PushNumber
            0xF4, // Opcode: ImmShort
            0x00, 0x01, // Operand: 1
            0x15, // Opcode: PushString
            0xF0, // Opcode: ImmStringByte
            0x00, // Operand: 0
            0x1b, // Opcode: PushPi
            0x07, // Opcode: Ret
        ]);
        let loader = BytecodeLoaderBuilder::new(reader).build().unwrap();

        assert_eq!(loader.function_map.len(), 2);
        assert_eq!(loader.function_map.get(&Some("main".to_string())), Some(&0));
        assert_eq!(loader.strings.len(), 1);
        assert_eq!(loader.strings.first(), Some(&"abc".to_string()));
        assert_eq!(loader.instructions.len(), 5);
        assert_eq!(loader.instructions[0].opcode, crate::opcode::Opcode::Jmp);
        assert_eq!(
            loader.instructions[1].opcode,
            crate::opcode::Opcode::PushNumber
        );
        assert_eq!(
            loader.instructions[1].operand,
            Some(crate::operand::Operand::new_number(1))
        );
        assert_eq!(
            loader.instructions[2].opcode,
            crate::opcode::Opcode::PushString
        );
        assert_eq!(
            loader.instructions[2].operand,
            Some(crate::operand::Operand::new_string("abc"))
        );
        assert_eq!(loader.instructions[3].opcode, crate::opcode::Opcode::Pi);
        assert_eq!(loader.instructions[4].opcode, crate::opcode::Opcode::Ret);
    }

    #[test]
    fn test_fmt_section_type() {
        assert_eq!(
            format!("{}", super::SectionType::Gs1Flags),
            "Gs1Flags".to_string()
        );
        assert_eq!(
            format!("{}", super::SectionType::Functions),
            "Functions".to_string()
        );
        assert_eq!(
            format!("{}", super::SectionType::Strings),
            "Strings".to_string()
        );
        assert_eq!(
            format!("{}", super::SectionType::Instructions),
            "Instructions".to_string()
        );
    }
}
