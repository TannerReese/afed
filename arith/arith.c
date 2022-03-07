#include <stdlib.h>
#include <math.h>

#include "arith.h"

struct arith_s {
	double num;
};

const char *arith_strerror(arith_err_t err){
	switch(err){
		case ARITH_ERR_OK: return "ARITH_ERR_OK: Successful";
	}
	
	return "ARITH_ERR: Unknown Error";
}



// Create deep copy of value, Allocating new memory
arith_t arith_clone(arith_t val){
	arith_t new = malloc(sizeof(struct arith_s));
	new->num = val->num;
	return new;
}

// Destroy value by deallocating memory
void arith_free(arith_t val){
	free(val);
}

// Parse value from string
arith_t arith_parse(const char *str, const char **endptr){
	const char *end = str;
	double val = strtod(str, (char**)&end);
	if(endptr) *endptr = end;
	if(end == str) return NULL;
	
	// Create memory location for result
	arith_t res = malloc(sizeof(struct arith_s));
	res->num = val;
	return res;
}

// Print value to stream pointer
int arith_print(FILE *stream, arith_t val){
	fprintf(stream, "%lf", val->num);
}



// Convert arith_t to double
double arith_todbl(arith_t val){
	return val->num;
}



// Unary Operation Implementation(s)
ARITH_FUNC(arith_neg){ args[0]->num = -(args[0]->num);  return args[0]; }
// Binary Operation Implementation(s)
ARITH_FUNC(arith_add){ args[0]->num += args[1]->num;  return args[0]; }
ARITH_FUNC(arith_sub){ args[0]->num -= args[1]->num;  return args[0]; }
ARITH_FUNC(arith_mul){ args[0]->num *= args[1]->num;  return args[0]; }
ARITH_FUNC(arith_div){ args[0]->num /= args[1]->num;  return args[0]; }
ARITH_FUNC(arith_flrdiv){ args[0]->num = floor(args[0]->num / args[1]->num);  return args[0]; }
ARITH_FUNC(arith_mod){ args[0]->num = fmod(args[0]->num, args[1]->num);  return args[0]; }
ARITH_FUNC(arith_pow){ args[0]->num = pow(args[0]->num, args[1]->num);  return args[0]; }


// Builtin Functions Implementation
ARITH_FUNC(arith_abs){ args[0]->num = fabs(args[0]->num); return args[0]; }
ARITH_FUNC(arith_floor){ args[0]->num = floor(args[0]->num); return args[0]; }
ARITH_FUNC(arith_ceil){ args[0]->num = ceil(args[0]->num); return args[0]; }
ARITH_FUNC(arith_sqrt){ args[0]->num = sqrt(args[0]->num); return args[0]; }
ARITH_FUNC(arith_log){ args[0]->num = log(args[0]->num) / log(args[1]->num); return args[0]; }
ARITH_FUNC(arith_ln){ args[0]->num = log(args[0]->num); return args[0]; }
ARITH_FUNC(arith_sin){ args[0]->num = sin(args[0]->num); return args[0]; }
ARITH_FUNC(arith_cos){ args[0]->num = cos(args[0]->num); return args[0]; }
ARITH_FUNC(arith_tan){ args[0]->num = tan(args[0]->num); return args[0]; }

// Constants
static arith_t arith_from(double val){
	arith_t ar = malloc(sizeof(struct arith_s));
	ar->num = val;
	return ar;
}

ARITH_FUNC(arith_PI){ return arith_from(3.14159265358979323846); }
ARITH_FUNC(arith_E){ return arith_from(2.71828182845904523536); }

