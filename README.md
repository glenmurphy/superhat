# Superhat
<img width="384" alt="superhat" src="https://github.com/user-attachments/assets/4428a76f-915b-440a-b131-f4e452425f55">

Planes in modern combat flight simulators have two or three MFDs - square displays that have five buttons (OSBs - Option Selection Buttons) on each side for a total of twenty per MFD. To access these buttons, a user must either use their mouse inside the sim, or buy multiple expensive hardware devices. In some situations it is also not convenient to have to take your hand off the joystick to interact with the MFDs. Superhat addresses this by allowing a single joystick 'hat' (a four-way button) to navigate the MFDs and press those OSBs.

## How it works
A long press to the left or right 'selects' which MFD is active.

A short press in any direction selects which edge of the MFD is selected, then you can select which of the five OSBs on that edge to press using further inputs - you can imagine the first press selects the middle OSB, then you can move the selection to adjacent OSBs by pushing in that direction. You confirm selection of the OSB by pushing in the direction you started - this way all OSBs can be accessed with 3 hat presses.

To use the right side as an example:
- to press OSB 8 (the middle one), press right>right
- to press OSB 7, press right>up>right
- to press OSB 6, press right>up>up (the final 'right' is not necessary as there are no other choices you can make)
- to press OSB 9, press right>down>right
- to press OSB 10, press right>down>down

The final press of the hatswitch presses the in-game OSB until you let go - this allows you to do short and long presses in-game.

## Setup
To use the software, download it from [the releases page](https://github.com/glenmurphy/superhat/releases) (expand the 'Assets' section under the latest version), run it, follow the binding instructions and enter your hat directions. Then launch your game and keep Superhat running in the background. When an OSB is pressed in Superhat, the software will emit the default Falcon BMS keyboard shortcuts for the OSBs. You can rebind your controls by pressing 'b' or clicking on the bind button.

If you have changed your BMS config, you can use the [/config/BMS - Superhat.key](./config/BMS%20-%20Superhat.key) file to restore the default bindings.

I recommend moving the F-16 DMS to a hat on the throttle if you have the space, and leaving a hat on the stick for Superhat. Remember to unbind any existing BMS controls on that hat.

### DCS setup
You will need to bind the keys that superhat emits - unfortunately this is pretty manual process of adding the modifiers:
- Left MFD OSB1-10: Ctrl+Alt+1,2,3..0
- Left MFD OSB11-20: Ctrl+Alt+Numpad1,Numpad2..0
- Right MFD OSB1-10: Ctrl+Shift+1,2,3...0
- Right MFD OSB11-20: Ctrl+Shift+Numpad1,Numpad2..0

## Feedback
Superhat is a prototype - please submit feedback via email to [glen@glenmurphy.com](mailto:glen@glenmurphy.com)
