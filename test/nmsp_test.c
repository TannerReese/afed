#include <stdio.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>

#include "../nmsp.h"

// Create namespace and declare variables printing any errors
namespace_t safe_decl(const char *decls[]);
// Parse and Evaluate the given definition
// Returns 1 if there is an unexpected parse or eval error
bool eval(namespace_t nmsp, const char *expstr, double tgt, parse_err_t perr, arith_err_t everr);
// Check that the stored dependency chain matches the given `names`
bool circ_loop(namespace_t nmsp, const char *chain);

#define sep()     puts("\n+-----------------+\n")
#define big_sep() puts("\n#=================#\n")

int check_parsing();
int check_func_parsing();
int check_parse_errs();
int check_insert_errs();

int main(int argc, char *argv[]){
	// Count number of failed tests
	int fails = 0;
	
	fails += check_parsing();
	big_sep();
	fails += check_func_parsing();
	big_sep();
	fails += check_parse_errs();
	big_sep();
	fails += check_insert_errs();
	big_sep();
	
	printf("\nFailures: %i\n", fails);
	return 0;
}



int check_parsing(){
	namespace_t nmsp;
	int fails = 0;
	puts("\n### Checking Parsing");
	
	const char *decls1[] = {
		"x :  \t-3.67",
		"y :1/ (x\n- z)",
		"z:1 /5.678- 2",
		NULL
	};
	if(!(nmsp = safe_decl(decls1))
	|| eval(nmsp,
		"(- \n x) ^-(y\n+z)*   x %\ty \t/ (z// 0.03)",
		0.0069547480181, PARSE_ERR_OK, PARSE_ERR_OK
	)) fails++;
	sep();
	
	// Make sure parser ignores extra content
	const char *decls2[] = {
		"x:5.32 * y",
		"foo_bar :y^3 - y^2-23",
		"y :  2.897 * 10^2",
		NULL
	};
	if(!(nmsp = safe_decl(decls2))
	|| eval(nmsp,
		"x *(foo_bar*x//y\v)//  -0.654=&*",
		-303764747679.0, PARSE_ERR_OK, PARSE_ERR_OK
	)) fails++;
	sep();
	
	const char *decls3[] = {
		"___:__-5*-5/-5%-3+4^1.3",
		"__:__OP*__OP         /4.5*__OP+3 -9.8-3",
		"__OP : \t4 + 6 + 8 - 9.4 - 4.56 + 3 / 5",
		NULL
	};
	if(!(nmsp = safe_decl(decls3))
	|| eval(nmsp,
		"___*__-__OP/__^__",
//		"___",
		204.122506542, PARSE_ERR_OK, PARSE_ERR_OK
	)) fails++;
	sep();
	
	// Make sure builtin functions and constants are parsed
	const char *decls4[] = {
		"xray:sin(ln(3.45 * pi) - stuff / beta)",
		"beta: 2 - abs(2 + stuff )^-2",
		"stuff :-4.356 * pi * log(e + 1, e - 1) = Ignored stuff",
		NULL
	};
	if(!(nmsp = safe_decl(decls4))
	|| eval(nmsp,
		"xray*beta + beta*stuff -stuff*xray",
		-61.39002848156, PARSE_ERR_OK, PARSE_ERR_OK
	)) fails++;
	
	return fails;
}

int check_func_parsing(){
	namespace_t nmsp;
	int fails = 0;
	puts("\n### Checking Function Parsing");
	
	const char *decls1[] = {
		"_ \t(\n\t__cfs\n,nj4X\n)\t :  (__cfs \n- nj4X/(__cfs ^-a__56JJ\n))",
		"__3NJr22(_,__,a__56JJ):_*__^(a__56JJ+_)-cos(__)",
		"  a__56JJ    :  -2.3\t* 7.8",
		NULL
	};
	if(!(nmsp = safe_decl(decls1))
	|| eval(nmsp,
		"_(abs(__3NJr22(a__56JJ,a__56JJ * _ \n( 1, 2), log(3,4))), sin(ln(abs(floor(a__56JJ^2*10)))))",
		-6139.752640153787, PARSE_ERR_OK, EVAL_ERR_OK
	)) fails++;
	
	const char *decls2[] = {
		"    my_Func(t):t - x  * 5*x",
		"\ttwoArg(x ,\n y) :x - y *y^ceil(x)",
		"x   : 4.5 - 3.2+31^2",
		NULL
	};
	if(!(nmsp = safe_decl(decls2))
	|| eval(nmsp,
		"   my_Func(twoArg(1.23, ln(\v5.12)))/cos(x) - tan(x * 5.6)",
		-8222343.424436592, PARSE_ERR_OK, EVAL_ERR_OK
	)) fails++;
	
	return fails;
}



int check_parse_errs(){	
	int fails = 0;
	puts("\n### Checking Parse Errors");
	namespace_t nmsp = nmsp_new(true);
	
	// Check for correct parsing errors
	printf("Checking Parsing Errors\n");
	if(eval(nmsp,
		"x + y - + * z\t",
		0.0, PARSE_ERR_MISSING_VALUES, PARSE_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"x * y - (x y)",
		0.0, PARSE_ERR_MISSING_OPERS, PARSE_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"((x * y - z) + x * z",
		0.0, PARSE_ERR_PARENTH_MISMATCH, PARSE_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"(x * y - z % 6)) / 7.0 ",
		0.0, PARSE_ERR_PARENTH_MISMATCH, PARSE_ERR_OK
	)) fails++;
	
	nmsp_free(nmsp);
	return fails;
}

int check_insert_errs(){
	puts("\n### Checking Insertion Errors");
	
	// Declarations use forward declared "ler", "two", and "_5_"
	const char *decls[] = {
		"xruje : yjug*yjug^-_5_*yjug+2",
		"__er34:3*xruje + ler*6",
		"gt56y : __er34 * yjug*4",
		"yjug : 23*9+two+7/6//3.65^7*8",
		"__23 : ( 1 \n+\n HEllo) / 34.56",
		"HEllo: __er34 + gt56y",
		NULL
	};
	namespace_t nmsp = safe_decl(decls);
	if(!nmsp) return 6;
	
	int fails = 0;
	parse_err_t err;
	const char *endptr;
	
	// Check redefinition errors
	if(eval(nmsp,
		"__23 : \t(1 + xruje * 8) / 9",
		0.0, INSERT_ERR_REDEF, EVAL_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"gt56y : yjug //2\t^2",
		0.0, INSERT_ERR_REDEF, EVAL_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"HEllo : xruje * yjug ^ 3",
		0.0, INSERT_ERR_REDEF, EVAL_ERR_OK
	)) fails++;
	sep();
	
	
	// Check circular dependency errors
	const char chn1[] = "_5_ <- xruje <- __er34 <- HEllo <- __23 <- _5_";
	if(eval(nmsp,
		"_5_:23//__23",
		0.0, INSERT_ERR_CIRC, EVAL_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn1)) fails++;
	
	const char chn2[] = "ler <- __er34 <- ler";
	if(eval(nmsp,
		"ler:__er34-73",
		0.0, INSERT_ERR_CIRC, EVAL_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn2)) fails++;
	
	const char chn3[] = "two <- yjug <- gt56y <- HEllo <- two";
	if(eval(nmsp,
		"two:(1+(2*(HEllo%4)+3)/4)//5",
		0.0, INSERT_ERR_CIRC, EVAL_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn3)) fails++;
	
	return fails;
}




namespace_t safe_decl(const char *decls[]){
	// Create namespace
	namespace_t nmsp = nmsp_new(true);
	
	// Declare all variables and parse their expressions
	const char *endptr;
	parse_err_t err;
	for(const char **dcl = decls; *dcl; dcl++){
		printf("Defining \"%s\"\n", *dcl);
		
		err = PARSE_ERR_OK;
		var_t vr = nmsp_define(nmsp, *dcl, &endptr, &err);
		printf("Consumed %u character(s) ; End-Pointer: \"%s\"\n", (size_t)(endptr - *dcl), endptr);
		printf("Parsing Errno: %i\n", err);
		printf("Variable Pointer: %p\n", vr);
		
		if(err || !vr){
			puts(err ? "**** Failed to Parse Expression" : "**** Failed to Define Variable");
			nmsp_free(nmsp);  // Cleanup namespace
			return NULL;
		}
		putchar('\n');
	}
	return nmsp;
}

bool circ_loop(namespace_t nmsp, const char *chain){
	// Iterate through dependency chain
	char buf[strlen(chain) + 1];
	nmsp_strcirc(nmsp, buf, strlen(chain) + 1);
	if(strcmp(buf, chain) != 0){
		printf("**** Dependency Chain doesn't match; Should be \"%s\" found \"%s\"\n", chain, buf);
		return 1;
	}
	
	printf("Dependency Chain matches \"%s\"\n", chain);
	return 0;
}

bool eval(namespace_t nmsp, const char *expstr, double tgt, parse_err_t perr, arith_err_t everr){
	// Parse expression
	const char *endptr;
	parse_err_t err;
	printf("Defining Expression \"%s\"\n", expstr);
	var_t vr = nmsp_define(nmsp, expstr, &endptr, &err);
	printf("Consumed %u character(s) ; End-Pointer: \"%s\"\n", (size_t)(endptr - expstr), endptr);
	printf("Desired Errno: %i     Parsing Errno: %i\n", perr, err);
	printf("Variable Pointer: %p\n", vr);
	if(perr != err){
		puts("**** Failed to Parse Expression");
		return 1;
	}
	
	// Evaluate expression
	if(!err && vr){
		printf("\nEvaluating Expression\n");
		
		arith_err_t err;
		arith_t res = nmsp_var_value(vr, &err);
		printf("Desired Errno: %i     Eval Errno: %i\n", everr, err);
		printf("Result Pointer: %p\n", res);
		printf("Desired Result: %.8lf     Result: %.8lf\n", tgt, arith_todbl(res));
		if(everr != err){
			puts("**** Failed to Evaluate Expression");
			return 1;
		}else if(fabs(tgt - arith_todbl(res)) > 0.00001){
			puts("**** Failed to Evaluate Expression Correctly");
			return 1;
		}
	}
	
	puts("Succeeded");
	return 0;
}

