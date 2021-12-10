#include <stdio.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>

#include "../expr.h"
#include "../expr_dbl.h"

// Type for expression string to be parsed
typedef const char *estr_t;
// Type for variable names
typedef const char *name_t;

// Declaration for variable name with its expression
typedef struct {
	// Null-terminated variable name
	const char *name;
	// Null-terminated expression string
	const char *expr;
} decl_t;

// Create namespace and declare variables printing any errors
namespace_t safe_decl(int deccnt, decl_t decls[]);
// Parse and Evaluate the given definition
// Returns 1 if there is an unexpected parse or eval error
bool eval(namespace_t nmsp, const char *expstr, double tgt, expr_err_t perr, expr_err_t everr);
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
	
	decl_t decls1[3] = {
		{"x", "  \t-3.67"},
		{"y", "1/ (x\n- z)"},
		{"z", "1 /5.678- 2"}
	};
	if(!(nmsp = safe_decl(3, decls1))
	|| eval(nmsp,
		"(- \n x) ^-(y\n+z)*   x %\ty \t/ (z// 0.03)",
		0.0069547480181, EXPR_ERR_OK, EXPR_ERR_OK
	)) fails++;
	sep();
	
	// Make sure parser ignores extra content
	decl_t decls2[3] = {
		{"x", "5.32 * y"},
		{"foo_bar", "y^3 - y^2-23"},
		{"y", "2.897 * 10^2"}
	};
	if(!(nmsp = safe_decl(3, decls2))
	|| eval(nmsp,
		"x *(foo_bar*x//y\v)//  -0.654=&*",
		-303764747679.0, EXPR_ERR_OK, EXPR_ERR_OK
	)) fails++;
	sep();
	
	decl_t decls3[3] = {
		{"___", "__-5*-5/-5%-3+4^1.3"},
		{"__", "__OP*__OP         /4.5*__OP+3 -9.8-3"},
		{"__OP", "4 + 6 + 8 - 9.4 - 4.56 + 3 / 5"}
	};
	if(!(nmsp = safe_decl(3, decls3))
	|| eval(nmsp,
		"___*__-__OP/__^__",
		204.122506542, EXPR_ERR_OK, EXPR_ERR_OK
	)) fails++;
	
	return fails;
}

int check_parse_errs(){	
	int fails = 0;
	puts("\n### Checking Parse Errors");
	namespace_t nmsp = nmsp_new(4);
	
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
	decl_t decls[6] = {
		{"xruje", "yjug*yjug^-_5_*yjug+2"},
		{"__er34", "3*xruje + ler*6"},
		{"gt56y", "__er34 * yjug*4"},
		{"yjug", "23*9+two+7/6//3.65^7*8"},
		{"__23", " ( 1 \n+\n HEllo) / 34.56"},
		{"HEllo", "__er34 + gt56y"}
	};
	namespace_t nmsp = safe_decl(6, decls);
	if(!nmsp) return 6;
	
	int fails = 0;
	expr_err_t err;
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
	const char chn1[] = "_5_ -> xruje -> __er34 -> HEllo -> __23";
	if(eval(nmsp,
		"_5_:23//__23",
		0.0, INSERT_ERR_CIRC, EXPR_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn1)) fails++;
	
	const char chn2[] = "ler -> __er34";
	if(eval(nmsp,
		"ler:__er34-73",
		0.0, INSERT_ERR_CIRC, EXPR_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn2)) fails++;
	
	const char chn3[] = "two -> yjug -> gt56y -> HEllo";
	if(eval(nmsp,
		"two:(1+(2*(HEllo%4)+3)/4)//5",
		0.0, INSERT_ERR_CIRC, EXPR_ERR_OK
	)) fails++;
	else if(circ_loop(nmsp, chn3)) fails++;
	
	return fails;
}




namespace_t safe_decl(int deccnt, decl_t decls[]){
	// Create namespace
	namespace_t nmsp = nmsp_new(4);
	
	// Declare all variables and parse their expressions
	const char *endptr;
	expr_err_t err;
	for(size_t i = 0; i < deccnt; i++){
		printf("Creating Variable \"%s\"\n", decls[i].name);
		
		// Parse expression that defines variable
		printf("Parsing Expression \"%s\"\n", decls[i].expr);
		expr_t exp = expr_parse(decls[i].expr, &endptr, nmsp, &err);
		printf("Consumed %u character(s) ; End-Pointer: \"%s\"\n", (size_t)(endptr - decls[i].expr), endptr);
		printf("Parsing Errno: %i\n", err);
		printf("Expression Pointer: %p\n", exp);
		if(err || !exp){
			nmsp_free(nmsp);  // Cleanup namespace
			printf("Failed to Parse Expression\n");
			return NULL;
		}
		
		// Add parsed expression to namespace
		printf("Inserting Expression\n");
		var_t vr = nmsp_insertz(nmsp, decls[i].name, exp);
		if(!vr){
			nmsp_free(nmsp);  // Cleanup namespace
			printf("Failed to declare Variable\n");
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
		printf("Dependency Chain doesn't match; Should be \"%s\" found \"%s\"\n", chain, buf);
		return 1;
	}
	
	printf("Dependency Chain matches \"%s\"\n", chain);
	return 0;
}

bool eval(namespace_t nmsp, const char *expstr, double tgt, expr_err_t perr, expr_err_t everr){
	// Parse expression
	const char *endptr;
	expr_err_t err;
	printf("Defining Expression \"%s\"\n", expstr);
	var_t vr = nmsp_define(nmsp, expstr, &endptr, &err);
	printf("Consumed %u character(s) ; End-Pointer: \"%s\"\n", (size_t)(endptr - expstr), endptr);
	printf("Desired Errno: %i     Parsing Errno: %i\n", perr, err);
	printf("Variable Pointer: %p\n", vr);
	if(perr != err){
		puts("Failed to Parse Expression");
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
			puts("Failed to Evaluate Expression");
			return 1;
		}else if(fabs(tgt - res) > 0.00001){
			puts("Failed to Evaluate Expression Correctly");
			return 1;
		}
	}
	
	puts("Succeeded");
	return 0;
}

