#ifndef __QUEUE_H
#define __QUEUE_H

#include <stddef.h>

// Queue of void pointers stored as circular buffer
typedef struct queue_s {
	// Pointer to memory allocated for queue
	void **ptr;
	
	// Offset of first element in queue from `ptr`
	size_t start;
	// Number of elements in queue & Maximum number possible
	size_t len, cap;
} *queue_t;


struct queue_s queue_new(size_t cap);
void queue_free(struct queue_s q);

void queue_push(queue_t q, void **ptrs, size_t ptrlen);
void *queue_pop(queue_t q);

#endif
