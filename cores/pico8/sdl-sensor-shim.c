/*
 * Preloaded under PICO-8 by System/pico8.sh. PICO-8 asks SDL_Init for every subsystem, sensors
 * among them, and the mali-fbdev SDL2 that reaches the H700's panel and knows its pad is built
 * without sensors: SDL_Init fails whole and PICO-8 stops before drawing anything. The handhelds
 * have no sensors for it anyway, so the sensor bit is taken out of what it asks for. As
 * spruceOS's Emu/PICO8/src/sdl_sensor_shim.c does.
 */
#define _GNU_SOURCE
#include <dlfcn.h>

#define SDL_INIT_SENSOR 0x00008000u

typedef int (*init_fn)(unsigned int);

static int forward(const char *name, unsigned int flags)
{
	init_fn real = (init_fn)dlsym(RTLD_NEXT, name);
	return real ? real(flags) : -1;
}

int SDL_Init(unsigned int flags)
{
	return forward("SDL_Init", flags & ~SDL_INIT_SENSOR);
}

int SDL_InitSubSystem(unsigned int flags)
{
	/* sensors alone are asked for on purpose, and fail as they would */
	return forward("SDL_InitSubSystem", flags == SDL_INIT_SENSOR ? flags : flags & ~SDL_INIT_SENSOR);
}
