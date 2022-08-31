#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{vec, vec::Vec};
use alloc::boxed::Box;


#[derive(Copy, Clone, Debug)]
pub struct Event {
    value: i32,
}

#[derive(Copy, Clone, Debug)]
pub struct Connection {
    cell_id: u16,
    port: u16,
}

pub struct PinInput {
    pin: u16,
    connections: Vec<Connection>,
}

/// Implement Primitive trait for HW pins. They are primitives in the sense that they can dispatch
/// events
impl Primitive for PinInput {
    
    fn dispatch(&mut self, _port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->()) {
        for c in &self.connections {
            output(*c, event);
        }
    }
}

struct SoftwareInput {
    addr: u16,
    connections: Vec<Connection>,
}

/// Implement Primitive trait for HW pins. They are primitives in the sense that they can dispatch
/// events
impl Primitive for SoftwareInput {
    fn dispatch(&mut self, _port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->()) {
        for c in &self.connections {
            output(*c, event);
        }
    }
}

pub trait Primitive {
    fn init(&mut self, _output: &mut dyn FnMut(Connection, Event)) { }
    fn dispatch(&mut self, port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->());
}

enum InferPrimitiveError {
    WrongPortCount,
    BadType,
    WrongParamCount
}

fn build_primitive(d: PrimitiveDescriptor) 
    -> Result<Box<dyn Primitive>, InferPrimitiveError> 
{
    println!("Building {:?}", d);
    let boxed_cell: Box<dyn Primitive> = match PrimitiveType::from(d.typecode) {
        PrimitiveType::Mux => Box::new(Mux::try_from(d)?),
        PrimitiveType::Demux => Box::new(Demux::try_from(d)?),
        PrimitiveType::Levels => Box::new(Levels::try_from(d)?),
        PrimitiveType::Bool => Box::new(Bool::try_from(d)?),
        PrimitiveType::Invalid => return Err(InferPrimitiveError::BadType),
    };
    Ok(boxed_cell)
}

#[derive(Clone, Debug)]
struct PrimitiveDescriptor {
    typecode: u16,
    out_ports: Vec<Vec<Connection>>,
    params: Vec<i32>,
}

#[repr(u16)]
enum PrimitiveType {
    Levels = 0,
    Mux = 1,
    Demux = 2,
    Bool = 3,
    Invalid = 0xffff,
}

impl From<u16> for PrimitiveType {
    fn from(x: u16) -> Self {
        use PrimitiveType::*;
        match x {
            0 => Levels,
            1 => Mux,
            2 => Demux,
            3 => Bool,
            _ => Invalid
        }
    }
}

struct Mux {
    select: usize,
    output: Vec<Connection>,
    inputs: Vec<i32>,
}

impl TryFrom<PrimitiveDescriptor> for Mux {
    type Error = InferPrimitiveError;

    fn try_from(mut value: PrimitiveDescriptor) -> Result<Self, Self::Error> {
        if value.typecode != PrimitiveType::Mux as u16 {
            return Err(InferPrimitiveError::BadType);
        }

        // Mux needs exactly 1 param
        if value.params.len() != 1 {
            return Err(InferPrimitiveError::WrongParamCount);
        }

        // A mux should have exactly 1 output
        if value.out_ports.len() != 1 {
            return Err(InferPrimitiveError::WrongPortCount);
        }

        return Ok(Mux {select: 0, output: value.out_ports.remove(0), inputs: vec![0; value.params[0] as usize]});
    }
}

impl Primitive for Mux {
    fn dispatch(&mut self, port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->()) {
        if port as usize > self.inputs.len() {
            return
        }

        if port as usize == self.inputs.len() {
            // Select port
            self.select = event.value as usize;
        } else {
            self.inputs[port as usize] = event.value;
        }

        if (self.select as usize) < self.inputs.len() {
            let event = Event { value: self.inputs[self.select]};
            println!("Mux firing {}", event.value);
            for c in &self.output {
                output(*c, event);
            }
        }
    }
}

struct Demux {
    select: u32,
    outputs: Vec<Vec<Connection>>,
}

impl TryFrom<PrimitiveDescriptor> for Demux {
    type Error = InferPrimitiveError;

    fn try_from(value: PrimitiveDescriptor) -> Result<Self, Self::Error> {
        if value.typecode != PrimitiveType::Demux as u16 {
            return Err(InferPrimitiveError::BadType);
        }

        // Demux expects no params
        if value.params.len() != 0 {
            return Err(InferPrimitiveError::WrongParamCount);
        }

        Ok(Demux { select: 0, outputs: value.out_ports })
    }
}

impl Primitive for Demux {
    fn dispatch(&mut self, port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->()) {
        println!("Demux {} on {}", event.value, port);
        if port == 0 {
            if (self.select as usize) < self.outputs.len() {

                for c in &self.outputs[self.select as usize] {
                    output(*c, event);
                }            
            }
        } else if port == 1 {
            self.select = event.value as u32;
        }
    }
}

struct Levels {
    levels: Vec<i32>,
    select: u32,
    output: Vec<Connection>,
}

impl TryFrom<PrimitiveDescriptor> for Levels {
    type Error = InferPrimitiveError;

    fn try_from(mut value: PrimitiveDescriptor) -> Result<Self, Self::Error> {
        if value.typecode != PrimitiveType::Levels as u16 {
            return Err(InferPrimitiveError::BadType);
        }

        // Levels has exactly one output
        if value.out_ports.len() != 1 {
            return Err(InferPrimitiveError::WrongPortCount)
        }

        Ok(Levels { levels: value.params, select: 0, output: value.out_ports.remove(0) })
    }
}

impl Primitive for Levels {

    fn init(&mut self, output: &mut dyn FnMut(Connection, Event)) {

        if self.levels.len() == 0 {
            return;
        }
        for c in &self.output {
            output(*c, Event { value: self.levels[0]});
        }
    }
    fn dispatch(&mut self, port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->()) {
        // "Bang" events require non-zero value to be valid
        println!("Levels {} on {}", event.value, port);
        if event.value == 0 {
            return;
        }
        if port == 0 { // Increment
            self.select = (self.select + 1) % self.levels.len() as u32;
        } else if port == 1 { // Decrement
            self.select = (self.select - 1) % self.levels.len() as u32;
        } else {
            return // Don't fire any output event for invalid ports
        }
        let event = Event { value: self.levels[self.select as usize] };
        println!("Levels: {}", event.value);
        for c in &self.output {
            output(*c, event)
        }
    }
}

struct Bool {
    output: Vec<Connection>,
}

impl TryFrom<PrimitiveDescriptor> for Bool {
    type Error = InferPrimitiveError;

    fn try_from(mut value: PrimitiveDescriptor) -> Result<Self, Self::Error> {
        if value.typecode != PrimitiveType::Bool as u16 {
            return Err(InferPrimitiveError::BadType);
        }

        // Bool has exactly one output
        if value.out_ports.len() != 1 {
            return Err(InferPrimitiveError::WrongPortCount);
        }

        if value.params.len() != 0 {
            return Err(InferPrimitiveError::WrongParamCount);
        }

        Ok(Bool { output: value.out_ports.remove(0) })
    }
}

impl Primitive for Bool {
    fn dispatch(&mut self, port: u16, event: Event, output: &mut dyn FnMut(Connection, Event)->()) {
        let outval = if port == 0 { // SET
            1
        } else if port == 1 {
            0
        } else if port == 2 {
            event.value
        } else {
            return
        };

        for c in &self.output {
            output(*c, Event { value: outval });
        }
    }
}



#[derive(Clone, Debug)]
pub enum DecodingError {
    InsufficientBytes,
    BadPrimitive,
    CountTooLarge,
}

struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}


use byteorder::{ByteOrder, LittleEndian};
impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data: data, pos: 0 }
    }

    pub fn read_u16(&mut self) -> Result<u16, DecodingError> {
        if self.remaining() < 2 {
            return Err(DecodingError::InsufficientBytes);
        }
        let result = LittleEndian::read_u16(&self.data[self.pos..]);
        self.pos += 2;
        return Ok(result);
    }

    pub fn read_u32(&mut self) -> Result<u32, DecodingError> {
        if self.remaining() < 4 {
            return Err(DecodingError::InsufficientBytes);
        }
        let result = LittleEndian::read_u32(&self.data[self.pos..]);
        self.pos += 4;
        return Ok(result);
    }

    pub fn read_i32(&mut self) -> Result<i32, DecodingError> {
        if self.remaining() < 4 {
            return Err(DecodingError::InsufficientBytes);
        }
        let result = LittleEndian::read_i32(&self.data[self.pos..]);
        self.pos += 4;
        return Ok(result);
    }

    pub fn skip(&mut self, n: usize) -> Result<(), DecodingError> {
        if self.remaining() < n {
            Err(DecodingError::InsufficientBytes)
        } else {
            self.pos += n;
            Ok(())
        }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}

fn read_connection_list(rdr: &mut BinaryReader, n: usize) -> Result<Vec<Connection>, DecodingError> {
    let mut connections = Vec::with_capacity(n);
    for _ in 0..n {
        let cell_id = rdr.read_u16()?;
        let port = rdr.read_u16()?;
        connections.push(
            Connection {
                cell_id: cell_id, 
                port: port,
            }
        )
    }
    Ok(connections)
}

fn read_input_list<F>(rdr: &mut BinaryReader, n: usize, mut cb: F) -> Result<(), DecodingError> 
where
    F: FnMut(u16, Vec<Connection>) -> ()
{
    for _ in 0..n {
        let input_id = rdr.read_u16()?;
        let name_size = rdr.read_u16()?;
        println!("Name size: {}", name_size);
        // We have no use for the name
        rdr.skip(name_size as usize)?; 
        let n_connections = rdr.read_u16()?;
        let connections = read_connection_list(rdr, n_connections as usize)?;
        cb(input_id, connections);
    }

    Ok(())
}

pub struct EventSystem {
    software_ports: Vec<SoftwareInput>,
    input_ports: Vec<PinInput>,
    cells: Vec<Box<dyn Primitive>>,
}

// Impose some reasonable limits on how much data we will try to read
const MAX_SOFTWARE_PORTS: u16 = 256;
const MAX_INPUT_PORTS: u16 = 32;
const MAX_CELLS: u32 = 256;
const MAX_PARAMETERS: u16 = 256;
const MAX_OUTPUTS: u16 = 256;
const MAX_CONNECTIONS: u16 = 256;

#[macro_use]
extern crate std;

impl EventSystem {
    pub fn from_netlist(data: &[u8]) -> Result<EventSystem, DecodingError> {
        let mut rdr = BinaryReader::new(data);

        // Read list of hardware pin inputs
        let n_pin_input = rdr.read_u16()?;
        println!("Inputs pins: {}", n_pin_input);
        if n_pin_input > MAX_INPUT_PORTS {
            return Err(DecodingError::CountTooLarge);
        }
        let mut pin_inputs = Vec::with_capacity(n_pin_input as usize);
        read_input_list(&mut rdr, n_pin_input as usize, |id, connections| {
            pin_inputs.push(PinInput{ pin: id, connections: connections });
        })?;

        // Read list of software inputs, which are functionally the same as pin inputs
        let n_sw_input = rdr.read_u16()?;
        println!("Sw pins: {}", n_sw_input);
        if n_sw_input > MAX_SOFTWARE_PORTS {
            return Err(DecodingError::CountTooLarge);
        }
        let mut sw_inputs = Vec::with_capacity(n_sw_input as usize);
        read_input_list(&mut rdr, n_sw_input as usize, |id, connections| {
            sw_inputs.push(SoftwareInput{ addr: id, connections: connections });
        })?;

        // Read list of primitive cells
        let n_cells = rdr.read_u32()?;
        println!("Cells: {}", n_cells);
        if n_cells > MAX_CELLS {
            return Err(DecodingError::CountTooLarge);
        }
        let mut cells = Vec::with_capacity(n_cells as usize);
        for _ in 0..n_cells {
            let cell_type = rdr.read_u16()?;
            let n_params = rdr.read_u16()?;
            if n_params > MAX_PARAMETERS {
                return Err(DecodingError::CountTooLarge);
            }
            let mut params = Vec::with_capacity(n_params as usize);
            for _ in 0..n_params {
                params.push(rdr.read_i32()?);
            }
            let n_outputs = rdr.read_u16()?;
            println!("Cell outputs: {}", n_outputs);
            if n_outputs > MAX_OUTPUTS {
                return Err(DecodingError::CountTooLarge);
            }
            let mut outputs = Vec::with_capacity(n_outputs as usize);
            for _ in 0..n_outputs {
                let n_connections = rdr.read_u16()?;
                if n_connections > MAX_CONNECTIONS {
                    return Err(DecodingError::CountTooLarge)
                }
                let connections = read_connection_list(&mut rdr, n_connections as usize)?;
                outputs.push(connections);
            }

            let cell_descriptor = PrimitiveDescriptor { typecode: cell_type, out_ports: outputs, params: params };
            cells.push(
                build_primitive(cell_descriptor).map_err(|_e| { DecodingError::BadPrimitive })?
            );
        }

        Ok(EventSystem { 
            input_ports: pin_inputs,
            software_ports: sw_inputs,
            cells: cells 
        })
    }

    pub fn init<F>(&mut self, output: &mut F) 
    where F: FnMut(u16, Event)
    {
        let mut pending_messages = Vec::new();
        for c in &mut self.cells {
            c.init(&mut |connection, event| {
                pending_messages.push((connection, event));
            });
        }

        // Continue processing output message until no more remain
        while let Some(msg) = pending_messages.pop() {
            let (c, event) = msg;
            
            if c.cell_id == 0xFFFF {
                // Special case for output primitives which are built in
                output(c.port, event);
            } else {
                if let Some(target_cell) = self.cells.get_mut(c.cell_id as usize) {
                    target_cell.as_mut().dispatch(c.port, event, &mut |connection, event| {
                        pending_messages.push((connection, event));
                    })
                }
            }
        }
    }

    fn process_event<F>(cells: &mut Vec<Box<dyn Primitive>>, target: &mut dyn Primitive, value: i32, output: &mut F)
    where F: FnMut(u16, Event)
    {
        let mut pending_messages = Vec::new();

        // Dispatch the intial message, and collect any output messages triggered
        target.dispatch(0, Event{ value }, &mut |connection, event| {
            pending_messages.push((connection, event));
        });

        // Continue processing output message until no more remain
        while let Some(msg) = pending_messages.pop() {
            let (c, event) = msg;
            
            if c.cell_id == 0xFFFF {
                // Special case for output primitives which are built in
                output(c.port, event);
            } else {
                if let Some(target_cell) = cells.get_mut(c.cell_id as usize) {
                    target_cell.as_mut().dispatch(c.port, event, &mut |connection, event| {
                        pending_messages.push((connection, event));
                    })
                }
            }
            
        }
    }

    pub fn process_hw_event<'a, F>(&mut self, pin: u16, value: i32, output: &'a mut F) 
    where F: FnMut(u16, Event)
    {
        if let Some(idx) = self.input_ports.iter().position(|p| p.pin == pin) {
            let target_port = self.input_ports.get_mut(idx).unwrap();
            Self::process_event(&mut self.cells, target_port, value, output);
        }
    }

    pub fn process_sw_event<'a, F>(&mut self, addr: u16, value: i32, output: &'a mut F)
    where F: FnMut(u16, Event)
    {
        if let Some(idx) = self.software_ports.iter().position(|p| p.addr == addr) {
            let target_port = self.software_ports.get_mut(idx).unwrap();
            Self::process_event(&mut self.cells, target_port, value, output);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1_case1() {
        
        let netlist = std::fs::read("example1.bin").unwrap();
        //netlist_file.read_to_string(&mut netlist).unwrap();

        let mut dut = EventSystem::from_netlist(&netlist).unwrap();

        let mut output_values = [0i32; 8];

        // let mut output_closure = |port: u16, event: Event| {
        //     println!("Got event {} {}", port, event.value);
        //     assert!(port < 8);
        //     output_values[port as usize] = event.value;
        // };

        dut.init(&mut |port: u16, event: Event| {
            println!("Got event {} {}", port, event.value);
            assert!(port < 8);
            output_values[port as usize] = event.value;
        });

        dut.process_sw_event(10, 12, &mut |port: u16, event: Event| {
            println!("Got event {} {}", port, event.value);
            assert!(port < 8);
            output_values[port as usize] = event.value;
        });
        
        assert_eq!(output_values[3], 12);
        
        dut.process_hw_event(1, 1, &mut |port: u16, event: Event| {
            println!("Got event {} {}", port, event.value);
            assert!(port < 8);
            output_values[port as usize] = event.value;
        });
        
        assert_eq!(output_values[3], 0);

        // Press "ON" button
        dut.process_hw_event(0, 1, &mut |port: u16, event: Event| {
            println!("Got event {} {}", port, event.value);
            assert!(port < 8);
            output_values[port as usize] = event.value;
        });

        assert_eq!(output_values[3], 1000);

        // Press "ON" button
        dut.process_hw_event(0, 1, &mut |port: u16, event: Event| {
            println!("Got event {} {}", port, event.value);
            assert!(port < 8);
            output_values[port as usize] = event.value;
        });

        assert_eq!(output_values[3], 3000);


        // Press "OFF" button
        dut.process_hw_event(1, 1, &mut |port: u16, event: Event| {
            println!("Got event {} {}", port, event.value);
            assert!(port < 8);
            output_values[port as usize] = event.value;
        });

        assert_eq!(output_values[3], 0);

    }
}