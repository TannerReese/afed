#ifndef __EXPR_H
#define __EXPR_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <string.h>


// Forward declaration of expression type
struct expr_s;
typedef struct expr_s *expr_t;

// Error type returned on failure to parse or evaluate expression
typedef int expr_err_t;
/* Positive error codes may be used to indicate arithmetic errors
 * These may be returned by expr_opers[id].func.binary and expr_opers[id].func.unary
 */

// Below are reserved values of expr_err_t
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
// Errors returned after parsing produces an invalid expression
#define PARSE_ERR_TOO_MANY_VALUES (-25)
#define PARSE_ERR_MISSING_VALUES (-26)
#define PARSE_ERR_MISSING_OPERS (-27)



// Forward declaration of variable type
struct var_s;
typedef struct var_s *var_t;

const char *nmsp_var_name(var_t vr, size_t *len);
expr_t nmsp_var_expr(var_t vr);
expr_err_t nmsp_var_value(void *dest, var_t vr);


struct namespace_s;
typedef struct namespace_s *namespace_t;

// Create new empty namespace
namespace_t nmsp_new();
void nmsp_free(namespace_t nmsp);

// Try to get a variable with the given name
var_t nmsp_get(namespace_t nmsp, const char *key, size_t keylen);
// Create variable with given name but with no expression
var_t nmsp_put(namespace_t nmsp, const char *key, size_t keylen);
// Try to insert the given expression under the given key
var_t nmsp_insert(namespace_t nmsp, const char *key, size_t keylen, expr_t exp);

// Used after erroneous 
// Get next variable in dependency chain starting from base of circular dependency
var_t nmsp_next_dep(namespace_t nmsp);

// Namespace functions for null-terminated strings
#define nmsp_getz(nmsp, key) nmsp_get((nmsp), (key), strlen(key))
#define nmsp_putz(nmsp, key) nmsp_put((nmsp), (key), strlen(key))
#define nmsp_insertz(nmsp, key, exp) nmsp_insert((nmsp), (key), strlen(key), (exp))



#define OPER_LEFT_ASSOC 1  // Left Associativity:  a ~ b ~ c  --->  (a ~ b) ~ c
#define OPER_RIGHT_ASSOC 0  // Right Associativity:  a ~ b ~ c  --->  a ~ (b ~ c)

typedef uint8_t oper_t;
#define OPER_NULL 0xff  // Represents undefined or null operator

// Information used to define identify operators
struct oper_info_s {
	// String used to represent the operator
	const char *name;
	size_t namelen;
	
	// Precedence and Associatitivity info
	uint8_t prec : 7;
	uint8_t assoc : 1;
	uint8_t is_unary : 1;  // Whether the operator is unary
	
	// Function used to apply operation
	union {
		expr_err_t (*unary)(void *arg);
		expr_err_t (*binary)(void *arg1, void *arg2);
	} func;
};

// List of valid operations 
extern struct oper_info_s expr_opers[];


// Create expression with the given capacities for each section
expr_t expr_new(size_t varcap, size_t constcap, size_t instrcap);
// Deallocates memory allocated to expression and any constants it holds
void expr_free(expr_t exp);
// Create new expression whose value is the given variable
expr_t expr_new_var(var_t vr);
// Create new expression whose value is the given constant
expr_t expr_new_const(void *val);

// Combine `vr` onto expression `exp` using binary operator `op`
expr_t expr_binary_var(expr_t exp, var_t vr, oper_t op);
// Combine `val` onto expression `exp` using binary operator `op`
expr_t expr_binary_const(expr_t exp, void *val, oper_t op);

// Combine `src` expression onto `dest` using binary operator `op`
expr_t expr_binary(expr_t dest, expr_t src, oper_t op);
// Modify `exp` by applying unary operator `op`
expr_t expr_unary(expr_t exp, oper_t op);



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
} expr_valctl_t;

// Control functions used by expression evaluator
extern expr_valctl_t expr_valctl;

// Macros for working with values
// Define stack space for value
#define valdef(vl) uint8_t vl[expr_valctl.size];
// Move value from location `src` to `dest`
#define valmove(dest, src) memmove(dest, src, expr_valctl.size)
// Do deep copy of value from `src` into `dest`
#define valclone(dest, src) (expr_valctl.clone ? expr_valctl.clone(dest, src) : valmove(dest, src))
// Check if two values are equal
#define valequal(v1, v2) (expr_valctl.equal ? expr_valctl.equal(v1, v2) : memcmp(v1, v2, expr_valctl.size))
// Deallocate value
#define valfree(vl) if(expr_valctl.free) expr_valctl.free(vl)



// Maximum allowed variables on stack during evaluation
#define EXPR_EVAL_STACK_SIZE 256
// Evaluate expression
expr_err_t expr_eval(void *dest, expr_t exp);

// Parse as much as possible of the string as expression
expr_t expr_parse(const char *str, const char **endptr, namespace_t nmsp, expr_err_t *err);
// Flag used to indicate if constant expressions should be simplified while parsing
// Defaults to true
extern bool expr_eval_on_parse;

#endif

