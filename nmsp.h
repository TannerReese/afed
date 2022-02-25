#ifndef __NMSP_H
#define __NMSP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdio.h>

#include "arith/arith.h"



// Error type returned on failure to parse or evaluate expression
typedef int nmsp_err_t;
/* Positive error codes may be used to indicate arithmetic errors
 * These may be returned by expr_opers[id].func.binary and expr_opers[id].func.unary
 * EVAL_ERR_ARITH may be used for a general (unspecified) arithmetic error
 * Otherwise the string description for the error is provided by expr_arith_strerror
 */

#define NMSP_ERR_OK (0)

// Parsing Errors
#define NMSP_ERR_PARENTH_MISMATCH (-16)
/* NMSP_ERR_LOWPREC_UNARY:
 *  Occurs when a unary operator follows a binary operator of higher precedence
 *  And the binary operator is left associative
 */
#define NMSP_ERR_LOWPREC_UNARY (-17)
#define NMSP_ERR_ARITY_MISMATCH (-18)
#define NMSP_ERR_BAD_COMMA (-19)
#define NMSP_ERR_FUNC_NOCALL (-20)

// Errors returned after parsing produces an invalid expression
#define NMSP_ERR_MISSING_VALUES (-25)
#define NMSP_ERR_MISSING_OPERS (-26)
#define NMSP_ERR_EXTRA_CONT (-32)
// Errors returned on failed insertions
#define INSERT_ERR_REDEF (-64)
#define INSERT_ERR_CIRC (-65)

// Returns a string containing a description of the error
const char *nmsp_strerror(nmsp_err_t err);




// Forward declaration of variable type
struct var_s;
typedef struct var_s *var_t;

const char *nmsp_var_name(var_t vr, size_t *len);
arith_t nmsp_var_value(var_t vr, arith_err_t *errp);
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


// Flag used to indicate if constant expressions should be simplified while parsing
// Defaults to true
extern bool nmsp_eval_on_parse;

#endif

