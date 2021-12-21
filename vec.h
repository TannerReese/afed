#include <stdlib.h>

#define vec_t(type) struct { size_t len, cap; type *ptr; }
// Initialize vector using given size or inferring size from the type
#define vecinit_sz(vc, capac, sz) { (vc).len = 0;  (vc).cap = (capac);  (vc).ptr = malloc((vc).cap * (sz)); }
#define vecinit(vc, capac) vecinit_sz(vc, capac, sizeof(*((vc).ptr)))
// Deallocate vector memory
#define vecfree(vc) { (vc).len = 0;  (vc).cap = 0;  if((vc).ptr) { free((vc).ptr);  (vc).ptr = NULL; }}

// Increase vector capacity to support a single new element
#define vecinc_sz(vc, sz) if((vc).len >= (vc).cap){\
	if((vc).cap == 0) (vc).cap = 1;\
	(vc).cap <<= 1;\
	(vc).ptr = realloc((vc).ptr, (vc).cap * (sz));\
}
#define vecinc(vc) vecinc_sz(vc, sizeof(*((vc).ptr)))

// Push element onto `vc`
#define vecpush(vc, elem) { vecinc(vc);  (vc).ptr[(vc).len++] = (elem); }
// Push content of pointer onto `vc` assuming element is of size `sz`
#define vecpush_sz(vc, elmp, sz) { vecinc_sz(vc, sz);  memmove((void*)(vc).ptr + ((vc).len++) * (sz), (elmp), (sz)); }


// Pop last element
#define vecpop(vc) (*((vc).len > 0 ? (vc).ptr + (--((vc).len)) : NULL))
#define vecpop_sz(vc, sz) ((vc).len > 0 ? (void*)(vc).ptr + (--((vc).len)) * (sz) : NULL)

// Remove last `n` elements from vector
// Return pointer to beginning of removed section
#define vecremove(vc, n) ((n) > (vc).len ? NULL : (vc).ptr + ((vc).len -= (n)))
#define vecremove_sz(vc, n, sz) ((n) > (vc).len ? NULL : (vc).ptr + ((vc).len -= (n)) * (sz))

// Return pointer to last element or null if empty
#define veclast(vc) ((vc).len > 0 ? (vc).ptr + (vc).len - 1 : NULL)
#define veclast_sz(vc, sz) ((vc).len > 0 ? (vc).ptr + ((vc).len - 1) * (sz) : NULL)
// Check if vector is empty
#define vecempty(vc) ((vc).len == 0)

