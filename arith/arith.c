#include <stdlib.h>
#include <math.h>

#include "arith.h"

const char *arith_strerror(arith_err_t err){
	switch(err){
		case ARITH_ERR_OK: return "ARITH_ERR_OK: Successful";
	}
	
	return "ARITH_ERR: Unknown Error";
}



// Create deep copy of value, Allocating new memory
arith_t arith_clone(arith_t val){ return val; }

// Destroy value by deallocating memory
void arith_free(arith_t val){ return; }

// Parse value from string
arith_t arith_parse(const char *str, const char **endptr){
	arith_t val;
	const char *iend = str, *fend = str;
	if(endptr) *endptr = str;
	
	// Try to parse as integer
	long i = strtol(str, (char**)&iend, 10);
	// Try to parse as floating point
	double r = strtod(str, (char**)&fend);
	if(r == (double)i && iend != str){
		val.num = i;
		val.den = 1;
		val.type = ARITH_RATIO;
		if(endptr) *endptr = iend;
	}else if(fend != str){
		val.real = r;
		val.type = ARITH_REAL;
		if(endptr) *endptr = fend;
	}
	
	return val;
}

// Print value to stream pointer
int arith_print(FILE *stream, arith_t val){
	switch(val.type){
		case ARITH_REAL:
			return fprintf(stream, "%lf", val.real);
		case ARITH_RATIO:
			if(val.den == 0) return fprintf(stream, "1 / 0");
			else if(val.den == 1) return fprintf(stream, "%li", val.num);
			else return fprintf(stream, "%li / %lu", val.num, val.den);
	}
}



// Convert arith_t to double
double arith_todbl(arith_t val){
	switch(val.type){
		case ARITH_REAL: return val.real;
		case ARITH_RATIO: return (double)val.num / val.den;
	}
}



#define fst  (args[0])
#define snd  (args[1])
#define trd  (args[2])
#define toreal(val)  ((double)(val).num / (val).den)

// Unary Operation Implementation(s)
ARITH_FUNC(arith_neg){
	switch(fst.type){
		case ARITH_REAL: fst.real = -fst.real;
		break;
		case ARITH_RATIO: fst.num = -fst.num;
		break;
	}
	return fst;
}

// Combine small integers together
#define both(x, y) (((x) << 4) | (y))

static void simplify(arith_t *val){
	if(val->type != ARITH_RATIO) return;
	if(val->num == 0){
		val->den = 1;
		return;
	}else if(val->den == 0){
		val->num = 1;
		return;
	}
	
	// Apply Euclidean's GCD algorithm
	unsigned long tmp, a, b = val->den;
	if(val->num < 0) a = -val->num;
	else a = val->num;
	
	if(a > b){
		tmp = a;
		a = b;
		b = tmp;
	}
	
	while(a > 0){
		tmp = b % a;
		b = a;
		a = tmp;
	}
	
	val->num /= (long)b;
	val->den /= b;
}

// Binary Operation Implementation(s)
ARITH_FUNC(arith_add){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real += snd.real;
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real += toreal(snd);
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = toreal(fst) + snd.real;
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.num *= snd.den;
			fst.num += snd.num * (long)fst.den;
			fst.den *= snd.den;
			simplify(&fst);
		break;
	}
	return fst;
}

ARITH_FUNC(arith_sub){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real -= snd.real;
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real -= toreal(snd);
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = toreal(fst) - snd.real;
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.num *= snd.den;
			fst.num -= snd.num * (long)fst.den;
			fst.den *= snd.den;
			simplify(&fst);
		break;
	}
	return fst;
}

ARITH_FUNC(arith_mul){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real *= snd.real;
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real *= snd.num;
			fst.real /= snd.den;
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = toreal(fst) * snd.real;
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.num *= snd.num;
			fst.den *= snd.den;
			simplify(&fst);
		break;
	}
	return fst;
}

ARITH_FUNC(arith_div){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real /= snd.real;
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real *= snd.den;
			fst.real /= snd.num;
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = toreal(fst) / snd.real;
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.num *= snd.den;
			if(snd.num < 0){
				fst.num = -fst.num;
				fst.den *= (unsigned long)(-snd.num);
			}else fst.den *= (unsigned long)(snd.num);
			simplify(&fst);
		break;
	}
	return fst;
}

ARITH_FUNC(arith_flrdiv){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.num = (long)floor(fst.real / snd.real);
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real /= snd.num;
			fst.real *= snd.den;
			fst.num = (long)floor(fst.real);
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.num = (long)floor(fst.num / (snd.real * fst.den));
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.num = (long)floor((double)fst.num * snd.den / fst.den / snd.num);
		break;
	}
	
	fst.type = ARITH_RATIO;
	fst.den = 1;
	return fst;
}

ARITH_FUNC(arith_mod){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real = fmod(fst.real, snd.real);
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real = fmod(fst.real, toreal(snd));
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = fmod(toreal(fst), snd.real);
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.num *= snd.den;

			fst.num %= snd.num * (long)fst.den;
			fst.den *= snd.den;
			simplify(&fst);
		break;
	}
	return fst;
}


static arith_t int_pow(arith_t val, int pow){
	long num_step = val.num, num_pow = 1;
	unsigned long den_step = val.den, den_pow = 1;
	
	if(pow < 0){
		pow = -pow;
		if(val.num < 0){
			num_step = -(long)val.den;
			den_step = -val.num;
		}else{
			num_step = val.den;
			den_step = val.num;
		}
	}
	
	while(pow > 0){
		if(pow & 1){
			num_pow *= num_step;
			den_pow *= den_step;
		}
		
		num_step *= num_step;
		den_step *= den_step;
		pow >>= 1;
	}
	
	val.num = num_pow;
	val.den = den_pow;
	return val;
}

ARITH_FUNC(arith_pow){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real = pow(fst.real, snd.real);
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real = pow(fst.real, toreal(snd));
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = pow(toreal(fst), snd.real);
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			if(snd.den == 1){
				fst = int_pow(fst, snd.num);
			}else{
				fst.real = pow(toreal(fst), toreal(snd));
				fst.type = ARITH_REAL;
			}
		break;
	}
	return fst;
}


// Builtin Functions Implementation
ARITH_FUNC(arith_abs){
	switch(fst.type){
		case ARITH_REAL:
			fst.real = fabs(fst.real);
		break;
		case ARITH_RATIO:
			if(fst.num < 0) fst.num = -fst.num;
		break;
	}
	return fst;
}

ARITH_FUNC(arith_floor){
	switch(fst.type){
		case ARITH_REAL:
			fst.num = floor(fst.real);
		break;
		case ARITH_RATIO:
			fst.num = (long)floor(toreal(fst));
		break;
	}
	fst.den = 1;
	fst.type = ARITH_RATIO;
	return fst;
}

ARITH_FUNC(arith_ceil){
	switch(fst.type){
		case ARITH_REAL:
			fst.num = ceil(fst.real);
		break;
		case ARITH_RATIO:
			fst.num = (long)ceil(toreal(fst));
		break;
	}
	fst.den = 1;
	fst.type = ARITH_RATIO;
	return fst;
}

ARITH_FUNC(arith_sqrt){
	switch(fst.type){
		case ARITH_REAL:
			fst.real = sqrt(fst.real);
		break;
		case ARITH_RATIO:
			fst.real = sqrt(toreal(fst));
			fst.type = ARITH_REAL;
		break;
	}
	return fst;
}

ARITH_FUNC(arith_log){
	switch(both(fst.type, snd.type)){
		case both(ARITH_REAL, ARITH_REAL):
			fst.real = log(fst.real) / log(snd.real);
		break;
		case both(ARITH_REAL, ARITH_RATIO):
			fst.real = log(fst.real) / log(toreal(snd));
		break;
		case both(ARITH_RATIO, ARITH_REAL):
			fst.real = log(toreal(fst)) / log(snd.real);
			fst.type = ARITH_REAL;
		break;
		case both(ARITH_RATIO, ARITH_RATIO):
			fst.real = log(toreal(fst)) / log(toreal(snd));
			fst.type = ARITH_REAL;
		break;
	}
	return fst;
}

ARITH_FUNC(arith_ln){
	switch(fst.type){
		case ARITH_REAL:
			fst.real = log(fst.real);
		break;
		case ARITH_RATIO:
			fst.real = log(toreal(fst));
			fst.type = ARITH_REAL;
		break;
	}
	return fst;
}

ARITH_FUNC(arith_sin){
	switch(fst.type){
		case ARITH_REAL:
			fst.real = sin(fst.real);
		break;
		case ARITH_RATIO:
			fst.real = sin(toreal(fst));
			fst.type = ARITH_REAL;
		break;
	}
	return fst;
}

ARITH_FUNC(arith_cos){
	switch(fst.type){
		case ARITH_REAL:
			fst.real = cos(fst.real);
		break;
		case ARITH_RATIO:
			fst.real = cos(toreal(fst));
			fst.type = ARITH_REAL;
		break;
	}
	return fst;
}

ARITH_FUNC(arith_tan){
	switch(fst.type){
		case ARITH_REAL:
			fst.real = tan(fst.real);
		break;
		case ARITH_RATIO:
			fst.real = tan(toreal(fst));
			fst.type = ARITH_REAL;
		break;
	}
	return fst;
}


// Constants
static arith_t arith_from(double val){
	arith_t ar;
	ar.type = ARITH_REAL;
	ar.real = val;
	return ar;
}

ARITH_FUNC(arith_PI){ return arith_from(3.14159265358979323846); }
ARITH_FUNC(arith_E){ return arith_from(2.71828182845904523536); }

