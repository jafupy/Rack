#pragma once

#include <stdint.h>

typedef void (*rack_core_event_callback_t)(const char *json, void *context);

int32_t rack_core_start(const char *config_json, rack_core_event_callback_t callback, void *context);
char *rack_core_command(const char *command_json);
void rack_core_free_string(char *value);
void rack_core_stop(void);
