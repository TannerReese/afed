#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <math.h>

#include "expr_dbl.h"

#define fst(args) (((double*)args)[0])  // Get first argument
#define snd(args) (((double*)args)[1])  // Get second argument

// Unary operators
static expr_err_t neg_d(void *arg);

// Binary operators
static expr_err_t add_d(void *arg1, void *arg2);
static expr_err_t sub_d(void *arg1, void *arg2);
static expr_err_t mul_d(void *arg1, void *arg2);
static expr_err_t div_d(void *arg1, void *arg2);
static expr_err_t flrdiv_d(void *arg1, void *arg2);
static expr_err_t mod_d(void *arg1, void *arg2);
static expr_err_t pow_d(void *arg1, void *arg2);

// Builtin Functions
static expr_err_t abs_d(void *args);
static expr_err_t sqrt_d(void *args);
static expr_err_t log_d(void *args);
static expr_err_t ln_d(void *args);
static expr_err_t sin_d(void *args);
static expr_err_t cos_d(void *args);

// Constants
double pi_d = 3.14159265358979323846, e_d = 2.71828182845904523536;

// Define operators
struct oper_info_s expr_opers[] = {
	{"-", 1, 100, OPER_LEFT_ASSOC, 0, 1, { .unary = neg_d }},  // EXPR_NEG
	{"+", 1, 64, OPER_LEFT_ASSOC, 0, 2, { .binary = add_d }},  // EXPR_ADD
	{"-", 1, 64, OPER_LEFT_ASSOC, 0, 2, { .binary = sub_d}},  // EXPR_SUB
	{"*", 1, 96, OPER_LEFT_ASSOC, 0, 2, { .binary = mul_d}},  // EXPR_MUL
	{"/", 1, 96, OPER_LEFT_ASSOC, 0, 2, { .binary = div_d}},  // EXPR_DIV
	{"//", 2, 96, OPER_LEFT_ASSOC, 0, 2, { .binary = flrdiv_d}},  // EXPR_FLRDIV
	{"%", 1, 96, OPER_LEFT_ASSOC, 0, 2, { .binary = mod_d}},  // EXPR_MOD
	{"^", 1, 112, OPER_RIGHT_ASSOC, 0, 2, { .binary = pow_d}},  // EXPR_POW
	{"abs", 3, 0, 0, 1, 1, { .nary = abs_d }},
	{"sqrt", 4, 0, 0, 1, 1, { .nary = sqrt_d }},
	{"log", 3, 0, 0, 1, 2, { .nary = log_d }},
	{"ln", 2, 0, 0, 1, 1, { .nary = ln_d }},
	{"sin", 3, 0, 0, 1, 1, { .nary = sin_d }},
	{"cos", 3, 0, 0, 1, 1, { .nary = cos_d }},
	{"pi", 2, 0, 0, 1, 0, { .value = &pi_d }},
	{"e", 1, 0, 0, 1, 0, { .value = &e_d }},
	{0}
};

// No Arithmetic errors produced
const char *(*expr_arith_strerror)(expr_err_t err) = NULL;



// Unary Operation Implementation
static expr_err_t neg_d(void *arg){ fst(arg) = -fst(arg);  return EXPR_ERR_OK; }
// Binary Operation Implementation
static expr_err_t add_d(void *arg1, void *arg2){ fst(arg1) += fst(arg2);  return EXPR_ERR_OK; }
static expr_err_t sub_d(void *arg1, void *arg2){ fst(arg1) -= fst(arg2);  return EXPR_ERR_OK; }
static expr_err_t mul_d(void *arg1, void *arg2){ fst(arg1) *= fst(arg2);  return EXPR_ERR_OK; }
static expr_err_t div_d(void *arg1, void *arg2){ fst(arg1) /= fst(arg2);  return EXPR_ERR_OK; }
static expr_err_t flrdiv_d(void *arg1, void *arg2){ fst(arg1) = floor(fst(arg1) / fst(arg2));  return EXPR_ERR_OK; }
static expr_err_t mod_d(void *arg1, void *arg2){ fst(arg1) = fmod(fst(arg1), fst(arg2));  return EXPR_ERR_OK; }
static expr_err_t pow_d(void *arg1, void *arg2){ fst(arg1) = pow(fst(arg1), fst(arg2));  return EXPR_ERR_OK; }



// Builtin Functions Implementation
static expr_err_t abs_d(void *args){ fst(args) = fabs(fst(args));  return EXPR_ERR_OK; }
static expr_err_t sqrt_d(void *args){ fst(args) = sqrt(fst(args));  return EXPR_ERR_OK; }
static expr_err_t log_d(void *args){ fst(args) = log(fst(args)) / log(snd(args));  return EXPR_ERR_OK; }
static expr_err_t ln_d(void *args){ fst(args) = log(fst(args));  return EXPR_ERR_OK; }
static expr_err_t sin_d(void *args){ fst(args) = sin(fst(args));  return EXPR_ERR_OK; }
static expr_err_t cos_d(void *args){ fst(args) = cos(fst(args));  return EXPR_ERR_OK; }



// Define controls
static int equal_d(void *val1, void *val2);
static void *clone_d(void *dest, void *src);
static void *parse_d(void *dest, const char *str, const char **endptr);
static int print_d(FILE *stream, void *val);

expr_valctl_t expr_valctl = { sizeof(double), equal_d, NULL, clone_d, parse_d, print_d };

static int equal_d(void *val1, void *val2){ return fst(val1) == fst(val2); }
static void *clone_d(void *dest, void *src){ fst(dest) = fst(src); }

static void *parse_d(void *dest, const char *str, const char **endptr){
	if(!dest) return NULL;
	
	// Give something to endptr to point to if NULL
	const char *tmp_endptr;
	if(!endptr) endptr = &tmp_endptr;
	
	*endptr = str;
	double val = strtod(str, (char**)endptr);
	if(*endptr == str) return NULL;
	
	// Put value into destination
	fst(dest) = val;
	return dest;
}

static int print_d(FILE *stream, void *val){
	fprintf(stream, "%lf", fst(val));
}

