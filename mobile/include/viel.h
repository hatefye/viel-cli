#ifndef VIEL_H
#define VIEL_H

#include <stdint.h>

uint16_t viel_start(const char *host, const char *token);
void viel_set_token(const char *token);

#endif
