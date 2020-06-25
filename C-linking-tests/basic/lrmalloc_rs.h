#ifndef __LRMALLOC_RS_HEADER__
#define __LRMALLOC_RS_HEADER__

#include <stddef.h>

void* malloc(size_t size);
void* calloc(size_t count, size_t size);
void* realloc(void* ptr, size_t new_size);
void free(void* ptr);
void* aligned_alloc(size_t align, size_t size);

unsigned char check_override();


#endif
