# Superhat
<img width="385" alt="superhat" src="https://github.com/user-attachments/assets/c4a54373-c299-4636-af91-bbc63f567be9">

Planes in modern combat flight simulators have two or three MFDs - square displays that have five buttons (OSBs - Option Selection Buttons) on each side for a total of twenty. To access these buttons, a user must either use their mouse inside the sim, or buy multiple expensive hardware devices. Further, in some situations it is not convenient to have to take your hand off the joystick to press one of the OSBs.

Superhat allows a single joystick 'hat' (a four-way button) to navigate the MFDs and press the OSBs - a long press to the left or right 'selects' which MFD is active. Any further short press selects which edge of the MFD is selected, then you can select which of the five OSBs on that edge to press using further inputs - you can imagine the first press selects the middle OSB, then you can move the selection to adjacent OSBs by pushing in that direction. You confirm selection of the OSB by pushing in the direction you started - this way all OSBs can be accessed with 3 hat presses.

To use the right side as an example:
- to press OSB 8 (the middle one), press right>right
- to press OSB 7, press right>up>right
- to press OSB 6, press right>up>up (the final 'right' is not necessary as there are no other choices you can make)
- to press OSB 9, press right>down>right
- to press OSB 10, press right>down>down

The final press of the hatswitch presses the in-game OSB until you let go - this allows you to do short and long presses

To use the software, run it and press the 'b' key to enter binding mode, then enter your hat directions. When an OSB is pressed, the software will emit Falcon BMS keyboard shortcuts for the OSBs (you should be able to map these in DCS)
