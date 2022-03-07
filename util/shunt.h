#ifndef __SHUNT_H
#define __SHUNT_H

#include "mcode.h"

// On failure to shunt failure
typedef int parse_err_t;

#define PARSE_ERR_OK (0)

/* PARSE_ERR_PARENTH_MISMATCH:
 *  Open or Close parenthesis present
 *  without corresponding parenthesis.
 *  Ex:  (x - (y * z)
 */
#define PARSE_ERR_PARENTH_MISMATCH (1)

/* PARSE_ERR_ARITY_MISMATCH:
 *  Wrong number of arguments are
 *  are given to a function.
 *  Ex:  f(x, y, z) + 1
 *  Where f takes two arguments
 */
#define PARSE_ERR_ARITY_MISMATCH (2)

/* PARSE_ERR_BAD_COMMA:
 *  Comma is present in an inappropriate location.
 *  Ex:  x , y
 */
#define PARSE_ERR_BAD_COMMA (3)

/* PARSE_ERR_MISSING_VALUES:
 *  Missing values between operators
 *  Ex:  x + * y
 */
#define PARSE_ERR_MISSING_VALUES (4)

/* PARSE_ERR_MISSING_OPERS:
 *  Missing operators between values
 *  Ex:  x y - z
 */
#define PARSE_ERR_MISSING_OPERS (5)

/* PARSE_ERR_VAR_CALL:
 *  When a valid value precedes
 *  a parenthetical block.
 *  Ex:  x (1, 2, 3)
 *  Where x is not a function
 */
#define PARSE_ERR_VAR_CALL (6)

/* PARSE_ERR_FUNC_NOCALL:
 *  When a valid function is not followed
 *  by parentheses.
 *  Ex:  f + x
 *  Where f is a function
 */
#define PARSE_ERR_FUNC_NOCALL (7)

/* PARSE_ERR_LOWPREC_UNARY:
 *  Occurs when a unary operator follows
 *  a binary operator of higher precedence
 *  and the binary operator is left associative
 *  Ex:  a == ! b
 *  Where `==` has higher precedence than `!`
 */
#define PARSE_ERR_LOWPREC_UNARY (8)


struct shunt_s;
typedef struct shunt_s *shunt_t;

// Initializer and Destructor
shunt_t shunt_new(mcode_t code, bool try_eval, size_t opcap);
void shunt_free(shunt_t shn);

// Return true if last token was a value
bool shunt_was_last_val(shunt_t shn);

// Put open parenth, close parenth, or comma in shunting yard
parse_err_t shunt_open_parenth(shunt_t shn);
parse_err_t shunt_close_parenth(shunt_t shn);
parse_err_t shunt_put_comma(shunt_t shn);

// Clear all remaining operators from operator stack
parse_err_t shunt_clear(shunt_t shn);

// Put fixity operator in shunting yard
parse_err_t shunt_put_unary(shunt_t shn, arith_func_t func, int prec);
parse_err_t shunt_put_binary(shunt_t shn, arith_func_t func, int prec, bool left_assoc);

// Call builtin or user-defined function
parse_err_t shunt_func_call(shunt_t shn, int arity, arith_func_t func);
parse_err_t shunt_code_call(shunt_t shn, mcode_t callee);

// Load argument, constant, or variable
parse_err_t shunt_load_arg(shunt_t shn, int arg);
parse_err_t shunt_load_const(shunt_t shn, arith_t value);
parse_err_t shunt_load_var(shunt_t shn, mcode_t var);

#endif
