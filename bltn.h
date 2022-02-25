#ifndef __BLTN_H
#define __BLTN_H

#include <stdbool.h>
#include "arith/arith.h"

#define OPER_LEFT_ASSOC 1  // Left Associativity:  a ~ b ~ c  --->  (a ~ b) ~ c
#define OPER_RIGHT_ASSOC 0  // Right Associativity:  a ~ b ~ c  --->  a ~ (b ~ c)

// Unary and Binary Builtin Operators
struct bltn_oper_s {
	// Null-terminated string representing operator
	const char *name;
	
	// Precedence and Associatitivity info for operators
	unsigned int prec : 7, assoc : 1;
	bool is_unary : 1;  // Whether operator is unary
	
	// Function to perform operation
	arith_func_t func;
};

typedef const struct bltn_oper_s *bltn_oper_t;

// ALphanumerically Named Builtin (Constant or Function)
struct bltn_s {
	// Null-terminated string representing builtin
	const char *name;
	
	/* For functions, this is the number of arguments
	 * For constants, this is 0
	 */
	int arity;
	
	/* Function to perform
	 * Or generator for constant
	 */
	arith_func_t func;
};

typedef const struct bltn_s *bltn_t;

// Try to parse `name` as non-operator builtin
// Returns NULL on no match
bltn_t bltn_parse(const char *name, size_t namelen);

// Try to parse `str` as operator
// Returns NULL on no match
bltn_oper_t bltn_oper_parse(const char *str, const char **endptr, bool is_unary);

#endif

