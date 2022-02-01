#include <stdlib.h>
#include <string.h>

#include "queue.h"


// Create queue with given capacity and initial content
struct queue_s queue_new(size_t cap){
	struct queue_s q;
	q.ptr = malloc(cap * sizeof(void*));
	q.start = 0;
	q.len = 0;
	q.cap = cap;
	return q;
}

void queue_free(struct queue_s q){
	free(q.ptr);
}



// Returns the new capacity
static size_t enlarge(queue_t q, size_t newlen){
	if(newlen > q->cap){
		size_t oldcap = q->cap;
		
		// Expand capacity until it fits the new length
		while(newlen > q->cap) q->cap <<= 1;
		// Allocate new capacity
		q->ptr = realloc(q->ptr, sizeof(void*) * q->cap);
		
		if(q->start + q->len > oldcap){
			// Move any discontiguous piece together
			memcpy(q->ptr + oldcap, q->ptr, (q->start + q->len - oldcap) * sizeof(void*));
		}

	}
	return q->cap;
}



// Add element to the end of the queue
void queue_push(queue_t q, void **ptrs, size_t ptrlen){
	// Make sure queue is long enough
	enlarge(q, q->len + ptrlen);
	
	// Find end of queue
	size_t end = q->start + q->len;
	end -= (end >= q->cap) * q->cap;
	
	// Increase number of elements
	q->len += ptrlen;
	// Copy elements
	int extra = (int)(end + ptrlen) - q->cap;
	if(extra > 0){
		// When new elements surpass end of buffer then wrap back around
		memcpy(q->ptr + end, ptrs, sizeof(void*) * (ptrlen - extra));
		memcpy(q->ptr, ptrs + (ptrlen - extra), sizeof(void*) * extra);
	}else{
		memcpy(q->ptr + end, ptrs, sizeof(void*) * ptrlen);
	}
}

// Remove element from the beginning of the queue
void* queue_pop(queue_t q){
	// Return NULL if no values in the queue
	if(q->len == 0) return NULL;
	
	void *p = q->ptr[q->start];
	q->start++;
	q->start -= (q->start >= q->cap) * q->cap;
	q->len--;
	return p;
}



