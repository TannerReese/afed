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

// Test expression for validity
static bool test(const char *expstr, double res, expr_err_t perr, expr_err_t everr, size_t deccnt, decl_t decls[]);

int main(int argc, char *argv[]){
	// Count number of failed tests
	int failures = 0;
	
	decl_t decls1[] = {
		{"x", "  \t-3.67"},
		{"y", "1\n/ (x- z)"},
		{"z", "1 /5.678- 2"}
	};
	if(test(
		"(-  x) ^-(y+z)*   x %\ny \t/ (z// 0.03)",
		0.0069547480181, EVAL_ERR_OK, EVAL_ERR_OK,
		3, decls1
	)) failures++;
	
	puts("\n===============\n");
	
	// Make sure parser ignores extra content
	decl_t decls2[] = {
		{"x", "5.32 * y"},
		{"foo_bar", "y^3 - y^2-23"},
		{"y", "2.897 * 10^2"}
	};
	if(test(
		"x *(foo_bar*x//y)\v//  -0.654=&*",
		-303764747679.0, EVAL_ERR_OK, EVAL_ERR_OK,
		3, decls2
	)) failures++;
	
	puts("\n===============\n");
	
	// Check for correct parsing errors
	printf("Checking Parsing Errors\n");
	if(test(
		"x + y - + * z\t",
		0.0, PARSE_ERR_MISSING_VALUES, EVAL_ERR_OK,
		0, NULL
	)) failures++;
	
	if(test(
		"x * y - (x y)",
		0.0, PARSE_ERR_MISSING_OPERS, EVAL_ERR_OK,
		0, NULL
	)) failures++;
	
	if(test(
		"((x * y - z) + x * z",
		0.0, PARSE_ERR_PARENTH_MISMATCH, EVAL_ERR_OK,
		0, NULL
	)) failures++;
	
	if(test(
		"(x * y - z % 6)) / 7.0 ",
		0.0, PARSE_ERR_PARENTH_MISMATCH, EVAL_ERR_OK,
		0, NULL
	)) failures++;
	puts("\n===============\n");
	
	printf("Failures: %i\n", failures);
	return 0;
}



static bool test(const char *expstr, double tgt, expr_err_t perr, expr_err_t everr, size_t deccnt, decl_t decls[]){
	// Create namespace
	namespace_t nmsp = nmsp_new();
	
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
			printf("Failed to Parse Expression\n");
			return 1;
		}
		
		// Add parsed expression to namespace
		printf("Inserting Expression\n");
		var_t vr = nmsp_insertz(nmsp, decls[i].name, exp);
		if(!vr){
			printf("Failed to declare Variable\n");
			return 1;
		}
		
		putchar('\n');
	}
	
	// Parse main expression
	printf("Parsing Main Expression \"%s\"\n", expstr);
	expr_t mainexp = expr_parse(expstr, &endptr, nmsp, &err);
	printf("Consumed %u character(s) ; End-Pointer: \"%s\"\n", (size_t)(endptr - expstr), endptr);
	printf("Desired Errno: %i     Parsing Errno: %i\n", perr, err);
	printf("Expression Pointer: %p\n", mainexp);
	if(perr != err){
		printf("Failed to Parse Main Expression\n");
		return 1;
	}
	
	// Evaluate main expression
	if(!err && !mainexp){
		printf("\nEvaluating Expression\n");
		void *res = expr_eval(mainexp, &err);
		printf("Desired Errno: %i     Eval Errno: %i\n", everr, err);
		printf("Result Pointer: %p\n", res);
		printf("Desired Result: %.8lf     Result: %.8lf\n", tgt, *(double*)res);
		if(everr != err){
			printf("Failed to Evaluate Expression\n");
			return 1;
		}else if(fabs(tgt - *(double*)res) > 0.00001){
			printf("Failed to Evaluate Expression Correctly\n");
			return 1;
		}
	}
	
	printf("Succeeded\n");
	return 0;
}
