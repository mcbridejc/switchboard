"""Generate an example system for light dimming

Basically, you get two buttons to drive a light output. The output controls the duty cycle of a PWM
or some other dimming mechanism.

The inputs are an "ON" button, and an "OFF" button. When OFF is pressed, the light should turn off
(output goes to 0). When ON is pressed, what happens depends on the current state: if the light is
off, it should turn back on to the brightness it had last time it was on, but if the light is
already on it should cycle to the next dimmer level.

There is also a software override port. This port allows another software controller (e.g. imagine
home assistant wants to turn off lights after a while) to set the light setting manually. An
override will take effect immediately, and will last until the buttons are pressed again.

NOTE: As I write this, I realize a flaw in the graph as designed: If the light state is on, and it
is turned off via the softare override, subsequently pressing ON will cause a change in dimmer level
rather than turning on to the prevous level.
"""

#%% 
import json
from switchboard import *

def TwoButtonDimmer(on, off, levels):
    demux = Demux(input=on)
    on_state = Bool(set=demux.output(0), reset=off)
    demux.sel = on_state.output
    dim_level = Levels(levels=levels, inc=demux.output(1))
    # Chooses between the dim level output, and 0 based on on_state
    onoff_mux = Mux(num_inputs=2, sel=on_state.output)
    onoff_mux.inputs[1] = dim_level.output
    # TODO: Need a primitive to initialize an input to a constant value
    # But luckily in this case we're OK because unconnected mux inputs will default to 0
    #onoff_mux.inputs[0] = Constant(0)

    return onoff_mux.output

light1_override = SoftwarePort("light1_set", 10)
light1_on_button = ButtonPort("light1_on", 0)
light1_off_button = ButtonPort("light1_off", 1)
light2_on_button = ButtonPort("light2_on", 2)
light2_off_button = ButtonPort("light2_off", 3)
light2_override = SoftwarePort("light2_set", 11)

# The output values which will be cycled through by the two output dimmer
DIMMING_LEVELS = [1000, 3000, 9000]
# Create a system with 8 outputs
system = EventGraph(8)
# Assign the lights to some arbitrary output channels
system.outputs[3] = join(light1_override, TwoButtonDimmer(light1_on_button, light1_off_button, DIMMING_LEVELS))
system.outputs[4] = join(light2_override, TwoButtonDimmer(light2_on_button, light2_off_button, DIMMING_LEVELS))
system.coalesce()

# Write JSON version of system
with open('example1.json', 'w') as f:
    f.write(json.dumps(system.to_dict()))

# Write binary version of system
encoded = machine_encode(system)
with open('example1.bin', 'wb') as f:
    f.write(encoded)
# %%
