#ifndef __NMSP_H
#define __NMSP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdio.h>

#include "arith/arith.h"
#include "util/shunt.h"

/* PARSE_ERR_EXTRA_CONT:
 *  When there are characters remaining
 *  after a valid expression is parsed.
 *  Ex:   x + y * z $$;;
 */
#define PARSE_ERR_EXTRA_CONT (31)

/* INSERT_ERR_REDEF:
 *  If a variable is given a name
 *  that has already been defined.
 *  Ex:  x : 3 - 1
 *       x : y + z
 */
#define INSERT_ERR_REDEF (32)

/* INSERT_ERR_CIRC:
 *  If a variable relies on itself via
 *  a chain of other variables.
 *  Ex:  y : z / 2
 *       x : (y * y) - y
 *       z : 3 + x
 */
#define INSERT_ERR_CIRC (33)

// Returns a string containing a description of the error
const char *nmsp_strerror(parse_err_t err);




struct var_s;
typedef struct var_s *var_t;

struct namespace_s;
typedef struct namespace_s *namespace_t;

// Get the name / value stored in variable
const char *nmsp_var_name(var_t vr, size_t *len);
arith_t nmsp_var_value(var_t vr, arith_err_t *errp);
// Print the value in var to the given stream
int nmsp_var_fprint(FILE *stream, var_t vr);

// Constructor and Destructor for Namespace
namespace_t nmsp_new(bool eval_on_parse);
void nmsp_free(namespace_t nmsp);

// Lookup variable with the given name
var_t nmsp_get(namespace_t nmsp, const char *key, size_t keylen);
#define nmsp_getz(nmsp, key) nmsp_get((nmsp), (key), strlen(key))

// Create variable with given name but with no expression
var_t nmsp_put(namespace_t nmsp, const char *key, size_t keylen);
#define nmsp_putz(nmsp, key) nmsp_put((nmsp), (key), strlen(key))

// Parse expression with label and try to insert it into the namespace
var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, parse_err_t *err);

// Used after erroneous `nmsp_insert` call

// Places string describing the circular dependency in `buf`
int nmsp_strcirc(namespace_t nmsp, char *buf, size_t sz);
// Places the name of the redefined variable in `buf`
int nmsp_strredef(namespace_t nmsp, char *buf, size_t sz);

#endif

