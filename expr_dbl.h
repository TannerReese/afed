#ifndef _EXPR_DOUBLE_H
#define _EXPR_DOUBLE_H

#include "expr.h"

// Define valid oper_t values
#define EXPR_NEG 0
#define EXPR_ADD 1
#define EXPR_SUB 2
#define EXPR_MUL 3
#define EXPR_DIV 4
#define EXPR_FLRDIV 5
#define EXPR_MOD 6
#define EXPR_POW 7

// Create container for double value
void *newdbl(double val);

// Create double expression constant
#define expr_new_dbl(val) expr_new_const(newdbl(val))

// Macros for common operations
#define expr_neg(exp) expr_unary(exp, EXPR_NEG)
#define expr_add(dest, src) expr_binary(dest, src, EXPR_ADD)
#define expr_sub(dest, src) expr_binary(dest, src, EXPR_SUB)
#define expr_mul(dest, src) expr_binary(dest, src, EXPR_MUL)
#define expr_div(dest, src) expr_binary(dest, src, EXPR_DIV)
#define expr_flrdiv(dest, src) expr_binary(dest, src, EXPR_FLRDIV)
#define expr_mod(dest, src) expr_binary(dest, src, EXPR_MOD)
#define expr_pow(dest, src) expr_binary(dest, src, EXPR_POW)

#define expr_add_var(exp, vr) expr_binary_var(exp, vr, EXPR_ADD)
#define expr_sub_var(exp, vr) expr_binary_var(exp, vr, EXPR_SUB)
#define expr_mul_var(exp, vr) expr_binary_var(exp, vr, EXPR_MUL)
#define expr_div_var(exp, vr) expr_binary_var(exp, vr, EXPR_DIV)
#define expr_flrdiv_var(exp, vr) expr_binary_var(exp, vr, EXPR_FLRDIV)
#define expr_mod_var(exp, vr) expr_binary_var(exp, vr, EXPR_MOD)
#define expr_pow_var(exp, vr) expr_binary_var(exp, vr, EXPR_POW)

#define expr_add_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_ADD)
#define expr_sub_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_SUB)
#define expr_mul_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_MUL)
#define expr_div_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_DIV)
#define expr_flrdiv_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_FLRDIV)
#define expr_mod_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_MOD)
#define expr_pow_dbl(exp, val) expr_binary_const(exp, newdbl(val), EXPR_POW)

#endif

