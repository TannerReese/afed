#include <stdlib.h>
#include <errno.h>
#include <math.h>

#include "expr_dbl.h"

// Create container for double value
void *newdbl(double val){
	double *valp = malloc(sizeof(double));
	*valp = val;
	return valp;
}

// Unary operators
expr_err_t neg_d(void *arg);

// Binary operators
expr_err_t add_d(void *arg1, void *arg2);
expr_err_t sub_d(void *arg1, void *arg2);
expr_err_t mul_d(void *arg1, void *arg2);
expr_err_t div_d(void *arg1, void *arg2);
expr_err_t flrdiv_d(void *arg1, void *arg2);
expr_err_t mod_d(void *arg1, void *arg2);
expr_err_t pow_d(void *arg1, void *arg2);

// Define operators
struct oper_info_s expr_opers[] = {
	{"-", 1, 100, OPER_LEFT_ASSOC, 1, { .unary = neg_d }},  // EXPR_NEG
	{"+", 1, 64, OPER_LEFT_ASSOC, 0, { .binary = add_d }},  // EXPR_ADD
	{"-", 1, 64, OPER_LEFT_ASSOC, 0, { .binary = sub_d}},  // EXPR_SUB
	{"*", 1, 96, OPER_LEFT_ASSOC, 0, { .binary = mul_d}},  // EXPR_MUL
	{"/", 1, 96, OPER_LEFT_ASSOC, 0, { .binary = div_d}},  // EXPR_DIV
	{"//", 2, 96, OPER_LEFT_ASSOC, 0, { .binary = flrdiv_d}},  // EXPR_FLRDIV
	{"%", 1, 96, OPER_LEFT_ASSOC, 0, { .binary = mod_d}},  // EXPR_MOD
	{"^", 1, 112, OPER_RIGHT_ASSOC, 0, { .binary = pow_d}},  // EXPR_POW
	{0}
};


expr_err_t neg_d(void *arg){
	*(double*)arg = -*(double*)arg;
	return EVAL_ERR_OK;
}

expr_err_t add_d(void *arg1, void *arg2){
	*(double*)arg1 += *(double*)arg2;
	return EVAL_ERR_OK;
}

expr_err_t sub_d(void *arg1, void *arg2){
	*(double*)arg1 -= *(double*)arg2;
	return EVAL_ERR_OK;
}

expr_err_t mul_d(void *arg1, void *arg2){
	*(double*)arg1 *= *(double*)arg2;
	return EVAL_ERR_OK;
}
expr_err_t div_d(void *arg1, void *arg2){
	*(double*)arg1 /= *(double*)arg2;
	return EVAL_ERR_OK;
}

expr_err_t flrdiv_d(void *arg1, void *arg2){
	*(double*)arg1 = floor(*(double*)arg1 / *(double*)arg2);
	return EVAL_ERR_OK;
}

expr_err_t mod_d(void *arg1, void *arg2){
	*(double*)arg1 = fmod(*(double*)arg1, *(double*)arg2);
	return EVAL_ERR_OK;
}

expr_err_t pow_d(void *arg1, void *arg2){
	*(double*)arg1 = pow(*(double*)arg1, *(double*)arg2);
	return EVAL_ERR_OK;
}



// Define controls
void free_d(void *val);
void *clone_d(void *val);
void *parse_d(const char *str, const char **endptr);

expr_valctl_t expr_valctl = { free_d, clone_d, parse_d };

void free_d(void *val){ free(val); }
void *clone_d(void *val){ return newdbl(*(double*)val); }

void *parse_d(const char *str, const char **endptr){
	// Give something to endptr to point to if NULL
	const char *tmp_endptr;
	if(!endptr) endptr = &tmp_endptr;
	
	*endptr = str;
	double val = strtod(str, (char**)endptr);
	if(*endptr == str) return NULL;
	
	// Allocate space if parsable
	double *valp = malloc(sizeof(double));
	*valp = val;
	return valp;
}

