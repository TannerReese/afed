#ifndef __ARITH_H
#define __ARITH_H

#include <stdio.h>

// Positive values are used for arithmetic errors
// Negative values are reserved for users of this header file
typedef int arith_err_t;
#define ARITH_ERR_OK 0

// Resolve arithmetic errors into strings
const char *arith_strerror(arith_err_t err);


// Type for value that can have operations performed on it
struct arith_s;
typedef struct arith_s *arith_t;
// Type for functions which handle values
typedef arith_t (*arith_func_t)(arith_t *args, arith_err_t *errp);
// Macro to create signature for arith_func_t functions
#define ARITH_FUNC(func) arith_t func(arith_t *args, arith_err_t *errp)

// Create deep copy of value, Allocating new memory
arith_t arith_clone(arith_t val);
// Destroy value by deallocating memory
void arith_free(arith_t val);
// Parse value from string
arith_t arith_parse(const char *str, const char **endptr);
// Print value to stream pointer
int arith_print(FILE *stream, arith_t val);

// Convert arith_t to double
double arith_todbl(arith_t val);


// Unary Operator
ARITH_FUNC(arith_neg);
// Binary Operator
ARITH_FUNC(arith_add);
ARITH_FUNC(arith_sub);
ARITH_FUNC(arith_mul);
ARITH_FUNC(arith_div);
ARITH_FUNC(arith_flrdiv);
ARITH_FUNC(arith_mod);
ARITH_FUNC(arith_pow);

// Builtin Functions
ARITH_FUNC(arith_abs);
ARITH_FUNC(arith_sqrt);
ARITH_FUNC(arith_log);
ARITH_FUNC(arith_ln);
ARITH_FUNC(arith_sin);
ARITH_FUNC(arith_cos);

// Constants
ARITH_FUNC(arith_PI);
ARITH_FUNC(arith_E);

#endif

