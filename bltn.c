#include <string.h>

#include "bltn.h"
#include "util/ptree.h"

struct bltn_oper_s builtin_opers[] = {
	{"-", 100, OPER_LEFT_ASSOC, true, arith_neg},
	{"+", 64, OPER_LEFT_ASSOC, false, arith_add},
	{"-", 64, OPER_LEFT_ASSOC, false, arith_sub},
	{"*", 96, OPER_LEFT_ASSOC, false, arith_mul},
	{"/", 96, OPER_LEFT_ASSOC, false, arith_div},
	{"//", 96, OPER_LEFT_ASSOC, false, arith_flrdiv},
	{"%", 96, OPER_LEFT_ASSOC, false, arith_mod},
	{"^", 112, OPER_RIGHT_ASSOC, false, arith_pow},
	{0}
};

struct bltn_s builtins[] = {
	{"abs", 1, arith_abs},
	{"floor", 1, arith_floor},
	{"ceil", 1, arith_ceil},
	{"sqrt", 1, arith_sqrt},
	{"log", 2, arith_log},
	{"ln", 1, arith_ln},
	{"sin", 1, arith_sin},
	{"cos", 1, arith_cos},
	{"tan", 1, arith_tan},
	{"pi", 0, arith_PI},
	{"e", 0, arith_E},
	{0}
};

bltn_t bltn_parse(const char *name, size_t namelen){
	// Search for Non-operators in array of Builtins
	for(struct bltn_s *bltn = builtins; bltn->name; bltn++){
		if(namelen == strlen(bltn->name)
		&& strncmp(bltn->name, name, namelen) == 0
		) return bltn;
	}
	return NULL;
}



// Root of operator trees
ptree_t unary_tree = ptree_new();
ptree_t binary_tree = ptree_new();

bltn_oper_t bltn_oper_parse(const char *str, const char **endptr, bool is_unary){
	// Select tree to use
	ptree_t *root = is_unary ? &unary_tree : &binary_tree;
	// Construct operator tree if it doesn't exist for given type
	if(!*root){
		// Add each operator to tree
		for(struct bltn_oper_s *oper = builtin_opers; oper->name; oper++){
			if(oper->is_unary == is_unary)
				ptree_put(root, oper->name, oper);
		}
	}
	
	// Set endptr to beginning for empty case
	if(endptr) *endptr = str;
	
	// Use operator tree to identify string
	return (bltn_oper_t)ptree_get(*root, str, endptr);
}

