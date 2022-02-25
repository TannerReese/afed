#ifndef __MCODE_H
#define __MCODE_H

#include <stddef.h>
#include <stdbool.h>

#include "arith/arith.h"

// arith_err_t values for Stack related errors
#define MCODE_ERR_OK (0)
#define MCODE_ERR_MISSING_ARGS (-1)
#define MCODE_ERR_UNKNOWN_INSTR (-2)
#define MCODE_ERR_STACK_SURPLUS (-3)
#define MCODE_ERR_UNDERFLOW (-4)
#define MCODE_ERR_INCOMPLETE_CODE (-5)

// Convert error code to string
const char *mcode_strerror(arith_err_t err);



// Type used to store sequence of instructions
struct mcode_s;
typedef struct mcode_s *mcode_t;

// Allocate & Deallocate code block
mcode_t mcode_new(int arity, size_t cap);
void mcode_free(mcode_t code);

// Allocate array of code blocks this one uses
// Return NULL when no use of outside code blocks
mcode_t *mcode_deplist(mcode_t code, size_t *lenp);

// Attempt to change the arity of a code block
// Returns 0 if arity successfully changed
// Returns -1 if arity is already set
// Otherwise minimum valid arity is returned
int mcode_set_arity(mcode_t code, int new_arity);
// Get current arity
int mcode_get_arity(mcode_t code);

// Return the stack height of code block
// Code block is only valid if Stack Height = 1
int mcode_stack_height(mcode_t code);

// Clear any cached values
// Returns true if there was a cached value
bool mcode_clear(mcode_t code);
// Remove all instructions and caching from code block
void mcode_reset(mcode_t code);
// Return cached error if present
arith_err_t mcode_error(mcode_t code);


/* Append instructions to the code block
 * Returns false on success, true otherwise
 */
bool mcode_load_const(mcode_t code, arith_t value);
bool mcode_load_arg(mcode_t code, int arg);
bool mcode_call_code(mcode_t code, mcode_t callee);
bool mcode_call_func(mcode_t code, int arity, arith_func_t func, bool try_eval);


// Execute the instructions in the code to get value
arith_t mcode_eval(mcode_t code, arith_t *args, arith_err_t *errp);

#endif

