Planes in modern combat flight simulators have two or three MFDs - square displays that have five buttons on each side (for a total of twenty buttons). To access these buttons, a user must either use their mouse inside the sim, or buy multiple expensive hardware devices.

My idea is that a single joystick 'hat' (a four-way button) can be used to navigate the MFDs - a long press to the left or right 'selects' which MFD is active. A subsequent short press in any four directions starts the button-pressing mode and selects which side of the MFD, then you can select which of the five buttons using subsequent inputs using a spatial model. This is best illustrated with an example:

- To select the right MFD, the user long presses the hat to the right.
- To select the top side of the MFD, the user presses the hat up.
- To press the left-most button, the user presses the hat left twice.
- To press the second button from the left, the user presses the hat left once then up once.
- To press the middle button, the user presses the hat up again.
- To press the button to the right of the middle button, the user presses the hat right once then up once.
- To press the right-most button, the user presses the hat right twice.

The button presses should spatially relative to the side - so if the user presses down to activate the bottom side, then down again should select the middle button. Similarly, if the user presses left to activate the left side, they should use left to activate the middle button, and up/down to choose the buttons up or down relative to that button

If a user presses the hat in a direction that is not one of the five cardinal directions, or waits for more than two seconds between presses, the button-pressing mode is cancelled (though the selected MFD remains active).

The button presses can be emitted as keyboard shortcuts. You do not have to implement this - please just print a message showing which button was pressed; for convience, consider the buttons as numbered 1-20 starting from the top-left and moving in a clockwise direction.

