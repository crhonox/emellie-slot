#!/bin/sh
# The pad of the handheld, for the programs launch_frontend.sh hands it over to: nds.sh and
# ports.sh source this. Sourced, not run.

# The sticks of each model, as spruceOS sorts the H700 handhelds; omega-power's PadLayout holds
# the same list for the controls omega-zero shows. A model BaseOS does not name has two.
pad_layout() {
	case "$1" in
	rg35xxsp | rg35xxplus | rg34xx | rgsp | rg28xx) echo nostick ;;
	rg40xxv) echo 1stick ;;
	*) echo 2stick ;;
	esac
}

# The pad as the mali SDL2 numbers it: the node's three phantom keys (ESC, VOL-, VOL+) come
# first, so A is b3; above MENU the stick clicks interleave with the triggers, one map for each
# layout. As spruceOS's AnbernicXXCommon.cfg, which measured them on an RG CubeXX.
pad_map() {
	base="19000000010000000100000000010000,ANBERNIC-keys,a:b3,b:b4,x:b6,y:b5,leftshoulder:b7,rightshoulder:b8,guide:b11,start:b10,back:b9,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2"
	case "$1" in
	nostick)
		echo "$base,lefttrigger:b12,righttrigger:b13,leftx:a0,lefty:a1,platform:Linux," ;;
	1stick)
		echo "$base,lefttrigger:b13,righttrigger:b14,leftx:a0,lefty:a1,leftstick:b12,platform:Linux," ;;
	*)
		echo "$base,lefttrigger:b13,righttrigger:b14,leftx:a0,lefty:a1,leftstick:b12,rightx:a2,righty:a3,rightstick:b15,platform:Linux," ;;
	esac
}

# pad_map's, by position (SDL's a is the bottom face button), so A/B and X/Y are swapped from
# the printed names, as spruceOS's export_sdl_gamecontroller_map positional does
pad_map_positional() {
	pad_map "$1" | sed -e 's/a:\([^,]*\),b:\([^,]*\)/a:\2,b:\1/' -e 's/x:\([^,]*\),y:\([^,]*\)/x:\2,y:\1/'
}
