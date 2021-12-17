#include <stdio.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>

#include "../nmsp.h"

// Create namespace and declare variables printing any errors
namespace_t safe_decl(const char *decls[]);
// Parse and Evaluate the given definition
// Returns 1 if there is an unexpected parse or eval error
bool eval(namespace_t nmsp, const char *expstr, double tgt, nmsp_err_t perr, nmsp_err_t everr);
// Check that the stored dependency chain matches the given `names`
bool circ_loop(namespace_t nmsp, const char *chain);

#define sep()     puts("\n+-----------------+\n")
#define big_sep() puts("\n#=================#\n")

// Check valid parsing
int check_parsing();
// Check errneous parsing
int check_parse_errs();
// Check insertion errors
int check_insert_errs();

int main(int argc, char *argv[]){
	// Count number of failed tests
	int fails = 0;
	
	fails += check_parsing();
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
		0.0069547480181, EXPR_ERR_OK, EXPR_ERR_OK
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
		-303764747679.0, EXPR_ERR_OK, EXPR_ERR_OK
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
		204.122506542, EXPR_ERR_OK, EXPR_ERR_OK
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
		-61.39002848156, EXPR_ERR_OK, EXPR_ERR_OK
	)) fails++;
	
	return fails;
}

int check_parse_errs(){	
	int fails = 0;
	puts("\n### Checking Parse Errors");
	namespace_t nmsp = nmsp_new();
	
	// Check for correct parsing errors
	printf("Checking Parsing Errors\n");
	if(eval(nmsp,
		"x + y - + * z\t",
		0.0, PARSE_ERR_MISSING_VALUES, EXPR_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"x * y - (x y)",
		0.0, PARSE_ERR_MISSING_OPERS, EXPR_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"((x * y - z) + x * z",
		0.0, PARSE_ERR_PARENTH_MISMATCH, EXPR_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"(x * y - z % 6)) / 7.0 ",
		0.0, PARSE_ERR_PARENTH_MISMATCH, EXPR_ERR_OK
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
	nmsp_err_t err;
	const char *endptr;
	
	// Check redefinition errors
	if(eval(nmsp,
		"__23 : \t(1 + xruje * 8) / 9",
		0.0, INSERT_ERR_REDEF, EXPR_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"gt56y : yjug //2\t^2",
		0.0, INSERT_ERR_REDEF, EXPR_ERR_OK
	)) fails++;
	
	if(eval(nmsp,
		"HEllo : xruje * yjug ^ 3",
		0.0, INSERT_ERR_REDEF, EXPR_ERR_OK
	)) fails++;
	sep();
	
	
	// Check circular dependency errors
	const char chn1[] = "_5_ <- xruje <- __er34 <- HEllo <- __23 <- _5_";
	if(eval(nmsp,
		"_5_:23//__23",
		0.0, INSERT_ERR_CIRC, EXPR_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn1)) fails++;
	
	const char chn2[] = "ler <- __er34 <- ler";
	if(eval(nmsp,
		"ler:__er34-73",
		0.0, INSERT_ERR_CIRC, EXPR_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn2)) fails++;
	
	const char chn3[] = "two <- yjug <- gt56y <- HEllo <- two";
	if(eval(nmsp,
		"two:(1+(2*(HEllo%4)+3)/4)//5",
		0.0, INSERT_ERR_CIRC, EXPR_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn3)) fails++;
	
	return fails;
}




namespace_t safe_decl(const char *decls[]){
	// Create namespace
	namespace_t nmsp = nmsp_new();
	
	// Declare all variables and parse their expressions
	const char *endptr;
	nmsp_err_t err;
	for(const char **dcl = decls; *dcl; dcl++){
		printf("Defining \"%s\"\n", *dcl);
		
		err = EXPR_ERR_OK;
		var_t vr = nmsp_define(nmsp, *dcl, &endptr, &err);
		printf("Consumed %u character(s) ; End-Pointer: \"%s\"\n", (size_t)(endptr - *dcl), endptr);
		printf("Parsing Errno: %i\n", err);
		printf("Expression Pointer: %p\n", exp);
		
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

bool eval(namespace_t nmsp, const char *expstr, double tgt, nmsp_err_t perr, nmsp_err_t everr){
	// Parse expression
	const char *endptr;
	nmsp_err_t err;
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
		double res;
		err = nmsp_var_value(&res, vr);
		printf("Desired Errno: %i     Eval Errno: %i\n", everr, err);
		printf("Result Pointer: %p\n", res);
		printf("Desired Result: %.8lf     Result: %.8lf\n", tgt, res);
		if(everr != err){
			puts("**** Failed to Evaluate Expression");
			return 1;
		}else if(fabs(tgt - res) > 0.00001){
			puts("**** Failed to Evaluate Expression Correctly");
			return 1;
		}
	}
	
	puts("Succeeded");
	return 0;
}

