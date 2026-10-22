#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "rc_api_runtime.h"
#include "rc_client.h"
#include "rc_hash.h"
#include "rc_libretro.h"

typedef struct omega_event_t {
  uint32_t type;
  uint32_t points;
  const char* title;
  const char* description;
  const char* badge_url;
  const char* message;
} omega_event_t;

typedef struct omega_core_memory_t {
  uint8_t* system_ram;
  size_t system_ram_size;
  uint8_t* save_ram;
  size_t save_ram_size;
} omega_core_memory_t;

typedef struct omega_game_t {
  const char* title;
  const char* badge_url;
  uint32_t console_id;
  uint32_t unlocked;
  uint32_t total;
} omega_game_t;

/* What retro_get_memory_data gave for the core's system RAM and save RAM, for a core that
 * declares no memory map (Snes9x): rc_libretro_memory_init asks for them only while it builds
 * the regions, on the thread that called omega_regions_new. */
static _Thread_local omega_core_memory_t omega_core_memory;

static void RC_CCONV omega_core_memory_info(uint32_t id, rc_libretro_core_memory_info_t* info) {
  info->data = NULL;
  info->size = 0;
  if (id == RETRO_MEMORY_SYSTEM_RAM) {
    info->data = omega_core_memory.system_ram;
    info->size = omega_core_memory.system_ram_size;
  } else if (id == RETRO_MEMORY_SAVE_RAM) {
    info->data = omega_core_memory.save_ram;
    info->size = omega_core_memory.save_ram_size;
  }
}

void* omega_regions_new(const struct retro_memory_descriptor* descriptors, unsigned count, uint32_t console_id,
                        const omega_core_memory_t* core_memory) {
  struct retro_memory_map map;
  rc_libretro_memory_regions_t* regions = (rc_libretro_memory_regions_t*)calloc(1, sizeof(*regions));
  if (!regions)
    return NULL;
  map.descriptors = descriptors;
  map.num_descriptors = count;
  memset(&omega_core_memory, 0, sizeof(omega_core_memory));
  if (core_memory)
    omega_core_memory = *core_memory;
  if (!rc_libretro_memory_init(regions, count ? &map : NULL, omega_core_memory_info, console_id)) {
    free(regions);
    return NULL;
  }
  return regions;
}

void omega_regions_free(void* regions) {
  if (!regions)
    return;
  rc_libretro_memory_destroy((rc_libretro_memory_regions_t*)regions);
  free(regions);
}

uint32_t omega_regions_read(const void* regions, uint32_t address, uint8_t* buffer, uint32_t num_bytes) {
  return rc_libretro_memory_read((const rc_libretro_memory_regions_t*)regions, address, buffer, num_bytes);
}

void omega_request_fields(const rc_api_request_t* request, const char** url, const char** post_data, const char** content_type) {
  *url = request->url;
  *post_data = request->post_data;
  *content_type = request->content_type;
}

void omega_respond(rc_client_server_callback_t callback, void* callback_data, const char* body, size_t body_length, int http_status_code) {
  rc_api_server_response_t response;
  response.body = body;
  response.body_length = body_length;
  response.http_status_code = http_status_code;
  callback(&response, callback_data);
}

void omega_event_fields(const rc_client_event_t* event, omega_event_t* out) {
  memset(out, 0, sizeof(*out));
  out->type = event->type;
  if (event->achievement) {
    out->title = event->achievement->title;
    out->description = event->achievement->description;
    out->badge_url = event->achievement->badge_url;
    out->points = event->achievement->points;
  }
  if (event->subset) {
    out->title = event->subset->title;
    out->badge_url = event->subset->badge_url;
  }
  if (event->server_error)
    out->message = event->server_error->error_message;
}

int omega_game_fields(rc_client_t* client, omega_game_t* out) {
  rc_client_user_game_summary_t summary;
  const rc_client_game_t* game = rc_client_get_game_info(client);
  memset(out, 0, sizeof(*out));
  if (!game || !game->id)
    return 0;
  rc_client_get_user_game_summary(client, &summary);
  out->title = game->title;
  out->badge_url = game->badge_url;
  out->console_id = game->console_id;
  out->unlocked = summary.num_unlocked_achievements;
  out->total = summary.num_core_achievements;
  return 1;
}

const char* omega_user_token(const rc_client_t* client) {
  const rc_client_user_t* user = rc_client_get_user_info(client);
  return user ? user->token : NULL;
}

const char* omega_user_name(const rc_client_t* client) {
  const rc_client_user_t* user = rc_client_get_user_info(client);
  return user ? user->display_name : NULL;
}

/* The default reader, for PlayStation discs as .cue with their .bin or .iso: set once, before
 * the first disc is hashed. */
void omega_hash_init_cdreader(void) {
  static int done;
  if (!done) {
    rc_hash_init_default_cdreader();
    done = 1;
  }
}

/* How many of the loaded game's achievements are still locked, or -1 with no game loaded. */
int omega_remaining(const rc_client_t* client) {
  rc_client_user_game_summary_t summary;
  const rc_client_game_t* game = rc_client_get_game_info(client);
  if (!game || !game->id)
    return -1;
  rc_client_get_user_game_summary(client, &summary);
  return summary.num_unlocked_achievements >= summary.num_core_achievements
             ? 0
             : (int)(summary.num_core_achievements - summary.num_unlocked_achievements);
}

/* The post data of a softcore unlock earned `seconds` ago, signed for that delay as rc_client
 * signs a retried one; 0 when `out` is too small or the request can't be built. */
size_t omega_award_post(const char* username, const char* token, uint32_t achievement_id, const char* game_hash,
                        uint32_t seconds, char* out, size_t cap) {
  rc_api_award_achievement_request_t params;
  rc_api_request_t request;
  size_t len = 0;
  memset(&params, 0, sizeof(params));
  params.username = username;
  params.api_token = token;
  params.achievement_id = achievement_id;
  params.hardcore = 0;
  params.game_hash = game_hash;
  params.seconds_since_unlock = seconds;
  if (rc_api_init_award_achievement_request(&request, &params) != RC_OK)
    return 0;
  if (request.post_data) {
    len = strlen(request.post_data);
    if (len < cap)
      memcpy(out, request.post_data, len + 1);
    else
      len = 0;
  }
  rc_api_destroy_request(&request);
  return len;
}
