#ifndef __NMSP_H
#define __NMSP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdio.h>



// Error type returned on failure to parse or evaluate expression
typedef int nmsp_err_t;
/* Positive error codes may be used to indicate arithmetic errors
 * These may be returned by expr_opers[id].func.binary and expr_opers[id].func.unary
 * EVAL_ERR_ARITH may be used for a general (unspecified) arithmetic error
 * Otherwise the string description for the error is provided by expr_arith_strerror
 */
#define EVAL_ERR_ARITH 1

// Below are reserved values of nmsp_err_t
#define EXPR_ERR_OK (0)
#define EVAL_ERR_STACK_OVERFLOW (-1)
#define EVAL_ERR_STACK_UNDERFLOW (-2)
#define EVAL_ERR_STACK_SURPLUS (-3)
#define EVAL_ERR_NO_EXPR (-4)
// Not returned by evaluation, only by parsing
#define PARSE_ERR_PARENTH_MISMATCH (-16)
/* PARSE_ERR_LOWPREC_UNARY:
 *  Occurs when a unary operator follows a binary operator of higher precedence
 *  And the binary operator is left associative
 */
#define PARSE_ERR_LOWPREC_UNARY (-17)
#define PARSE_ERR_ARITY_MISMATCH (-18)
#define PARSE_ERR_BAD_COMMA (-19)
#define PARSE_ERR_FUNC_NOCALL (-20)

// Errors returned after parsing produces an invalid expression
#define PARSE_ERR_TOO_MANY_VALUES (-25)
#define PARSE_ERR_MISSING_VALUES (-26)
#define PARSE_ERR_MISSING_OPERS (-27)
#define PARSE_ERR_EXTRA_CONT (-32)
// Errors returned on failed insertions
#define INSERT_ERR_REDEF (-64)
#define INSERT_ERR_CIRC (-65)

// Returns a string containing a description of the error
const char *nmsp_strerror(nmsp_err_t err);




// Forward declaration of variable type
struct var_s;
typedef struct var_s *var_t;

const char *nmsp_var_name(var_t vr, size_t *len);
nmsp_err_t nmsp_var_value(void *dest, var_t vr);
int nmsp_var_fprint(FILE *stream, var_t vr);


struct namespace_s;
typedef struct namespace_s *namespace_t;

// Create new empty namespace
namespace_t nmsp_new();
void nmsp_free(namespace_t nmsp);

// Try to get a variable with the given name
var_t nmsp_get(namespace_t nmsp, const char *key, size_t keylen);
#define nmsp_getz(nmsp, key) nmsp_get((nmsp), (key), strlen(key))
// Create variable with given name but with no expression
var_t nmsp_put(namespace_t nmsp, const char *key, size_t keylen);
#define nmsp_putz(nmsp, key) nmsp_put((nmsp), (key), strlen(key))

// Parse expression with label and try to insert it into the namespace
var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, nmsp_err_t *err);

// Used after erroneous `nmsp_insert` call

// Returns string describing the circular dependency
// NOTICE: Returned string must be freed
int nmsp_strcirc(namespace_t nmsp, char *buf, size_t sz);
// Returns a null-terminated name of the attempted to be redefined variable
// NOTICE: Returned string must be freed
int nmsp_strredef(namespace_t nmsp, char *buf, size_t sz);




#define OPER_LEFT_ASSOC 1  // Left Associativity:  a ~ b ~ c  --->  (a ~ b) ~ c
#define OPER_RIGHT_ASSOC 0  // Right Associativity:  a ~ b ~ c  --->  a ~ (b ~ c)

typedef uint8_t bltn_t;
#define OPER_NULL 0xff  // Represents undefined or null operator

/* Information used to define
 * builtin operators, functions, and constants
 * 
 * Builtin operators consist of `isoper` characters
 * and are called as
 *     <oper> <arg>
 * for Unary and
 *     <arg1> <oper> <arg2>
 * for Binary
 * 
 * Builtin functions consist of alphanumeric characters and '_'
 * They are called as
 *     <builtin_func>(<arg1>, <arg2>, ...)
 */
struct bltn_info_s {
	// String used to represent the operator
	const char *name;
	size_t namelen;
	
	// Precedence and Associatitivity info for operators
	uint8_t prec : 7;
	uint8_t assoc : 1;
	// True when builtin is a function or constant
	// Named using alphanumerics (and '_')
	uint8_t is_alpha : 1;
	
	/* For unary operators, this is 1
	 * For binary operators, this is 2
	 * For functions, this is the number of arguments
	 * For constants, this is 0
	 */
	uint8_t arity : 4;
	
	// Function or void pointer used to define behavior of builtin
	union {
		nmsp_err_t (*unary)(void *arg);
		nmsp_err_t (*binary)(void *arg1, void *arg2);
		
		// args is a pointer to an array of arguments
		nmsp_err_t (*nary)(void *args);
		void *value;  // Stores value of constant
	} src;
};

// Null-terminated array of builtin operators, functions, and constants
extern struct bltn_info_s nmsp_bltns[];
// Resolve arithmetic errors into strings
extern const char *(*expr_arith_strerror)(nmsp_err_t err);



// Functions used to manipulate values
typedef struct {
	// Size of value in bytes
	size_t size;
	
	// Check if two values are equal
	// NOTE: If null then memory comparison is done
	int (*equal)(void *val1, void *val2);
	
	// Deallocate an instance of a value
	// NOTE: If null no deallocation is necessary
	void (*free)(void *val);
	// Create a deep copy of a value and place it in dest
	// Should return destination pointer
	void *(*clone)(void *dest, void *src);
	
	// Parse value from string
	// Should return destination pointer
	void *(*parse)(void *dest, const char *str, const char **endptr);
	
	// Print value to stream pointer
	int (*print)(FILE *stream, void *val);
} nmsp_valctl_t;

// Control functions used by expression evaluator
extern nmsp_valctl_t nmsp_valctl;

// Macros for working with values
// Define stack space for value
#define valdef(vl) uint8_t vl[nmsp_valctl.size];
#define valarr_def(vl, num) uint8_t vl[(num) * nmsp_valctl.size];
// Move value from location `src` to `dest`
#define valmove(dest, src) memmove(dest, src, nmsp_valctl.size)
// Do deep copy of value from `src` into `dest`
#define valclone(dest, src) (nmsp_valctl.clone ? nmsp_valctl.clone(dest, src) : valmove(dest, src))
// Check if two values are equal
#define valequal(v1, v2) (nmsp_valctl.equal ? nmsp_valctl.equal(v1, v2) : memcmp(v1, v2, nmsp_valctl.size))
// Deallocate value
#define valfree(vl) if(nmsp_valctl.free) nmsp_valctl.free(vl)



// Flag used to indicate if constant expressions should be simplified while parsing
// Defaults to true
extern bool nmsp_eval_on_parse;

#endif

