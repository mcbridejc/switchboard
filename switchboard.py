from typing import List, Optional, Tuple
import struct

class InputPortProperty:
    def __set_name__(self, _owner, name):
        self.attrname = "_" + name
        
    def __get__(self, obj, objtype=None):
        return getattr(obj, self.attrname)
    
    def __set__(self, obj, value):
        if value is not None and not isinstance(value, OutputPort):
            raise ValueError(f"{self.attrname} can only be assigned an OutputPort object or None")
        setattr(obj, self.attrname, value)

class InputPortArray:
    def __init__(self, values):
        self.values = values

    def __getitem__(self, key):
        return self.values[key]
    
    def __setitem__(self, key, value):
        if value is not None and not isinstance(value, OutputPort):
            raise ValueError(f"Input ports can only be assigned an OutputPort object or None")
        self.values[key] = value

    def __len__(self):
        return len(self.values)

class OutputPort(object):
    def __init__(self, cell=None):
        self.targets = []
        self.cell = cell

    @property
    def in_ports(self):
        raise RuntimeError("in_ports not implemented")

    @property
    def cells(self):
        """Return driving cells as an array
        Most output ports belong to one cell, but this allows support for
        virtual outputs (i.e. Joiner) which are driven by multiple outputs
        """
        if self.cell is not None:
            return [self.cell]
        else:
            return []

    def register_target(self, target):
        if target not in self.targets:
            self.targets.append(target)

    def __repr__(self):
        return f"OutputPort(targets={self.targets}, cell={self.cell and self.cell.id})"

class SystemInput(OutputPort):
    def __init__(self):
        super().__init__()

    @property
    def in_ports(self):
        return []

class SoftwarePort(SystemInput):
    def __init__(self, name, addr):
        super().__init__()
        self.name = name
        self.addr = addr
        self.output = OutputPort(self)

    def to_dict(self):
        return {
            "name": self.name,
            "addr": self.addr,
            "drives": [{"cell": t[0], "port": t[1]} for t in self.targets],
        }

    def input_id(self):
        return self.addr

class ButtonPort(SystemInput):
    def __init__(self, name, pin):
        super().__init__()
        self.name = name
        self.pin = pin
        self.output = OutputPort(self)
    
    def to_dict(self):
        return {
            "name": self.name,
            "pin": self.pin,
            "drives": [{"cell": t[0], "port": t[1]} for t in self.targets],
        }

    def input_id(self):
        return self.pin

    @property
    def in_ports(self) -> List[Optional[OutputPort]]:
        return []

    @property
    def out_ports(self) -> List[OutputPort]:
        """Returns list of output ports
        Ordering is critical, as it defines the ordering in serialization formats
        """
        return [self.output]

class Primitive(object):
    id_map = {}
    def __init__(self):
        typename = self.type
        if typename not in Primitive.id_map:
            Primitive.id_map[typename] = 0
        
        Primitive.id_map[typename] += 1
        self.id = f"{typename}_{Primitive.id_map[typename]}"
    
    def to_dict(self):
        return {
            "id": self.id,
            "type": self.type,
            "outputs": [
                [{"cell": t[0], "port": t[1]} for t in out.targets]
                for out in self.out_ports
            ],
        }

    @property
    def type(self):
        """Return the canonical name for this type of primitive cell"""
        return self.__class__.__name__.lower()

    @property
    def type_code(self):
        """Machine code for this cell type
        Used for binary serialization to MCU
        """
        TYPE_CODES = [
            "levels",
            "mux",
            "demux",
            "bool"
        ]
        try:
            return TYPE_CODES.index(self.type)
        except ValueError:
            raise ValueError(f"No typecode found for primitive {self.type}")

    @property
    def parameters(self) -> List[int]:
        raise RuntimeError("Primitive must implement parameters property")

class Levels(Primitive):
    inc: OutputPort = InputPortProperty()
    dec: OutputPort = InputPortProperty()

    def __init__(self, levels: List[int], inc=None, dec=None):
        super().__init__()
        self.levels = levels
        self.inc = inc
        self.dec = dec
        self.output = OutputPort(self)
    
    @property
    def in_ports(self) -> List[OutputPort]:
        return [self.inc, self.dec]
    
    @property
    def out_ports(self) -> List[OutputPort]:
        return [self.output]

    @property
    def parameters(self) -> List[int]:
        return self.levels

class Mux(Primitive):
    sel: Optional[OutputPort] = InputPortProperty()

    def __init__(self, num_inputs=2, sel: OutputPort=None):
        super().__init__()
        self.sel = sel
        self._inputs: List[Optional[OutputPort]] = InputPortArray([None] * num_inputs)
        self.output = OutputPort(self)

    @property
    def inputs(self) -> List[Optional[OutputPort]]:
        return self._inputs

    @property
    def in_ports(self) -> List[OutputPort]:
        return self._inputs.values + [self.sel]
    
    @property
    def out_ports(self) -> List[OutputPort]:
        return [self.output]

    @property
    def parameters(self) -> List[int]:
        return [len(self._inputs)]

class Demux(Primitive):
    input: Optional[OutputPort] = InputPortProperty()
    sel: Optional[OutputPort] = InputPortProperty()

    def __init__(self, num_outputs=2, input: OutputPort=None, sel: OutputPort=None, ):
        super().__init__()
        self.input = input
        self.sel = sel
        self._outputs = [OutputPort(self) for _ in range(num_outputs)]

    def output(self, n):
        """Get the nth output of the demux"""
        if n >= len(self._outputs):
            raise ValueError(f"Attempt to access port {n} on {len(self._outputs)} demux")
        return self._outputs[n]

    @property
    def in_ports(self):
        return [self.input, self.sel]

    @property
    def out_ports(self):
        return self._outputs

    @property
    def parameters(self):
        return []

class Bool(Primitive):
    set: Optional[OutputPort] = InputPortProperty()
    reset: Optional[OutputPort] = InputPortProperty()
    assign: Optional[OutputPort] = InputPortProperty()

    def __init__(self, set: OutputPort=None, reset: OutputPort=None, assign: OutputPort=None):
        super().__init__()
        self.set = set
        self.reset = reset
        self.assign = assign
        self.value = 0
        self.output = OutputPort(self)

    @property
    def in_ports(self) -> List[OutputPort]:
        return [self.set, self.reset, self.assign]

    @property
    def out_ports(self) -> List[OutputPort]:
        return [self.output]
    
    @property
    def parameters(self):
        return []

class Joiner(OutputPort):
    """pseudo primitive which represents a connection to multiple outputs"""
    def __init__(self, outputs: List[OutputPort]):
        self.upstream_outputs = outputs
        self.cell = self

    def register_target(self, target: Tuple[str, int]):
        for o in self.upstream_outputs:
            o.register_target(target)

    @property
    def in_ports(self) -> List[OutputPort]:
        return self.upstream_outputs

    @property
    def cells(self):
        cells = []
        for op in self.upstream_outputs:
            if op.cell is not None and op.cell not in cells:
                cells.append(op.cell)
        return cells

def join(*nets):
    return Joiner(nets)



class EventGraph(object):
    def __init__(self, n_output: int):
        self.outputs:List[Optional[OutputPort]] = [None] * n_output
        self.software_ports: Optional[List[SoftwarePort]] = None
        self.button_ports: Optional[List[ButtonPort]] = None
        self.cells: Optional[List[Primitive]] = None

    def coalesce(self) -> Tuple[List[SoftwarePort], List[ButtonPort]]:
        """"""
        outputs = self.outputs
        software_ports = set()
        button_ports = set()
        primitives = set()
        # Keep track of registered cells to avoid infinite recursion from connection loops
        registered_cells = []

        def harvest_upstream_ports(inp):
            """Check ports if input port is a top level input, with special case for
            virtual joiner ports
            """
            ports = []
            # This special case is giving me second thoughts about this joiner pattern
            if isinstance(inp, Joiner):
                ports = inp.in_ports
            else:
                ports = [inp]
            
            for p in ports:
                if isinstance(p, SoftwarePort):
                    software_ports.add(p)
                if isinstance(p, ButtonPort):
                    button_ports.add(p)

        def register_inputs(cell):
            if cell in registered_cells:
                return
            registered_cells.append(cell)
            primitives.add(cell)
            for i, inp in enumerate(cell.in_ports):
                if inp is not None:
                    harvest_upstream_ports(inp)
                    inp.register_target((cell.id, i))
                    for c in inp.cells:
                        register_inputs(c)
        
        for i, out in enumerate(outputs):
            if out is not None:
                out.register_target(("out", i))
                harvest_upstream_ports(out)
                for c in out.cells:
                    register_inputs(c)

        self.software_ports = list(software_ports)
        self.button_ports = list(button_ports)
        self.cells = list(primitives)

    def to_dict(self):
        return {
            'software_ports': [p.to_dict() for p in self.software_ports],
            'button_ports': [p.to_dict() for p in self.button_ports],
            'cells': [p.to_dict() for p in self.cells],
        }


def machine_encode(sys: EventGraph):
    """Serialize an EventGraph to a binary format for programming to a device"""

    def find_cell_index(cell_id):
        """Get the cell index number for encoding
        
        A special cell value of 65535 is used for output
        """
        if cell_id == 'out':
            return 0xffff
        for i, c in enumerate(sys.cells):
            if c.id == cell_id:
                return i
        raise RuntimeError(f"Couldn't not find cell {cell_id}")

    def encode_inputs(inputs):
        """Serialize a set of input objects to the buffer
        
        Utility function because serializing software and hardware inputs is the same
        """
        buf = b''
        n_inputs = len(inputs)
        buf += struct.pack("<H", n_inputs)
        for input in inputs:
            name_size = len(input.name)
            num_connections = len(input.targets)
            # Format is <pin (u16)> <name_size (u16)> <name> <num_connections> <connection 0>...
            # Connection is <cell_index (u16)> <port (u16)>
            total_size = 2 + name_size + num_connections * 4
            buf += struct.pack(f"<HH{name_size}sH", input.input_id(), name_size, bytes(input.name, 'utf-8'), num_connections)
            for c in input.targets:
                # Convert the string name to index in array
                cell_index = find_cell_index(c[0])
                port = c[1]
                buf += struct.pack("<HH", cell_index, port)
        return buf

    buf = b''

    # Encode inputs
    buf += encode_inputs(sys.button_ports)
    buf += encode_inputs(sys.software_ports)
    
    # Encode primitive cells
    n_cells = len(sys.cells)
    buf += struct.pack("<L", n_cells)
    for cell in sys.cells:
        buf += struct.pack("<H", cell.type_code)
        n_params = len(cell.parameters)
        buf += struct.pack(f"<H", n_params)
        buf += struct.pack(f"{n_params}i", *cell.parameters)
        n_outputs = len(cell.out_ports)
        buf += struct.pack("<H", n_outputs)
        for p in cell.out_ports:
            n_connections = len(p.targets)
            buf += struct.pack("<H", n_connections)
            for t in p.targets:
                cell_index = find_cell_index(t[0])
                port = t[1]
                buf += struct.pack("<HH", cell_index, port)

    return buf


