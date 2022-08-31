Switchboard
===========

Discrete event simulator written in Rust, and a python module to generate event system graphs. 

## Summary

This project is designed to define simple graphs of connected primitives, which form a discrete
event model which can then be executed in an embedded device. It's just a prototype, and is probably
a terrible idea.

## A motivating example

Say you want to control some LED lights, with some buttons. Maybe you build a controller with some
LED drivers, and some button inputs. Also let's assume it might have some kind of connection to a
higher level controller (e.g. home assistant). You might want to define different ways for button
presses to control outputs, for example, you could have one button turn a light on and off, and two
other buttons dim it up and down. 

Now, you could build the controller to simply read buttons, pass them to its controller (e.g. home
assistant) and take commands to control the lights. But then it hassio is down your lights don't
switch. So that's kind of bad. Better that the basic rules are programmed to the controller and work
without any hassio in the loop. 

## What it does

It's kind of netlist, consisting of various primitives. Primitives are just blocks with a set of input ports, output ports, and some associated logic for how it drives outputs based on inputs. 

Ports are message based. Ports don't drive a level, like a circuit net. Outputs simply generate events which are passed to any connected inputs. Messages can be "bangs", which basically just means something like "the thing just happened", or they can have arbitrary values. In practice, a bang is just any non-zero value event, and whether it's interpreted as a "bang" or value just depends on the input port receiving it.

Events always originate with an external input, which can be received over a hardware input (i.e. a button press) or over a software port (i.e. a message was received from the higher level controller). This input ports connect to primitives, and probably cause those to generate more events in turn cascading through the graph and possibly causing an event to be received by one of the output ports, changing its state.

## What are the primitives

This is a good question, and this deserves more thought. As it stands, primitives must be defined in the python library, and also in the Rust simulator, so it would be nice to keep the primitives to a small well defined set that doesn't change much. Larger modules can be built out of primitives, so that graph designers don't have to think too much about the low level primitives (see `TwoButtonDimmer` in [example1.py](example1.py) for example).

Primitives can have parameters. For example, the size of a mux/demux is defined by a parameter.

Right now there are four primitives

- Bool: Holds a state. It has two bang inputs (set, reset) and an output (0 or 1). 
- Demux: Routes one input to one of multiple outputs. It has two inputs (input, sel), and N outputs.
- Mux: Passes messages only from one active input to the output. It has N inputs + sel, and one output.
- Levels: A dimmer, basically. This exists primarily because of my first example case. It takes a set of levels as parameters, and has an increment and decrement input port. Bangs on these ports cause the level output to change up or down in the list (wrapping as needed). This should probably be implemented as a counter that can go up/down, and a mux with constant inputs; that's more general.

