#include <stdlib.h>

#include "mcode.h"

#include "shunt.h"

/* Classes of tokens used to identify elements on operator stack
 * and determine missing values and operators
 */
enum token_type {
	TOKEN_PARENTH,  // Open parenthesis
	TOKEN_COMMA,
	TOKEN_FIXITY,  // Unary (prefix) or Binary (infix) operator
	TOKEN_MCODE_FUNC,  // User-defined function
	TOKEN_FUNC,  // Builtin function
	
	// Not used on operator stack
	TOKEN_VALUE  // Constant, Argument, Variable or Close parenthesis
};



// Element of operator stack
struct shn_oper_s {
	// Distinguish between parenthesis, comma, and others
	enum token_type type;
	
	/* High priority ops are shunted before low priority.
	 * So the Stack, from bottom to top, will have increasing priority.
	 */
	int priority;
	
	int arity; // Number of arguments operator takes
	
	union {
		// TOKEN_MCODE_FUNC: code block defining user-defined function
		mcode_t code;
		// TOKEN_UNARY, TOKEN_BINARY, TOKEN_FUNC: function pointer defining builtin
		arith_func_t func;
	};
};

// Shunting yard structure with value and operator stacks
struct shunt_s {
	bool try_eval : 1;  // Try to evaluate functions when parsing
	enum token_type last : 4;
	
	// Code Block is used as value stack
	mcode_t vals;
	
	// Operator stack
	size_t oplen, opcap;
	struct shn_oper_s *ops;
};

shunt_t shunt_new(mcode_t code, bool try_eval, size_t opcap){
	shunt_t shn = malloc(sizeof(struct shunt_s));
	shn->try_eval = try_eval;
	// Starting parsing with parenthesis causes no issues
	shn->last = TOKEN_PARENTH;
	
	shn->vals = code;
	shn->oplen = 0;  shn->opcap = opcap;
	shn->ops = malloc(opcap * sizeof(struct shn_oper_s));
	return shn;
}

void shunt_free(shunt_t shn){
	free(shn->ops);
	// Do not deallocate vals (i.e. code block)
	free(shn);
}

// Return true if last token was a value
bool shunt_was_last_val(shunt_t shn){
	return shn->last == TOKEN_VALUE;
}



/* Remove fixity operators with higher priority than `thresh`
 * from operator stack and apply them to the value stack
 * 
 * Returns true when error encountered
 */
static bool displace_fixity(shunt_t shn, int thresh){
	struct shn_oper_s *top = shn->ops + shn->oplen - 1;
	for(; top >= shn->ops && top->priority > thresh && top->type == TOKEN_FIXITY; top--){
		// Apply fixity operator to the value stack
		if(mcode_call_func(shn->vals, top->arity, top->func, shn->try_eval)) return true;
	}
	shn->oplen = top - shn->ops + 1;  // Remove displaced fixity operators
	return false;
}

static inline struct shn_oper_s *ops_inc(shunt_t shn){
	if(shn->oplen >= shn->opcap){  // Resize if necessary
		shn->opcap += shn->opcap == 0;  // Make sure capacity is at least one
		shn->opcap <<= 1;
		shn->ops = realloc(shn->ops, shn->opcap * sizeof(struct shn_oper_s));
	}
	return shn->ops + (shn->oplen++);  // Return pointer to new element
}



parse_err_t shunt_open_parenth(shunt_t shn){
	// Previous token must not be a value
	if(shn->last == TOKEN_VALUE) return PARSE_ERR_VAR_CALL;
	
	struct shn_oper_s *op = ops_inc(shn);
	op->type = TOKEN_PARENTH;
	op->priority = -1;
	op->arity = 0;
	shn->last = TOKEN_PARENTH;
	return PARSE_ERR_OK;
}

parse_err_t shunt_close_parenth(shunt_t shn){
	// Check that last token was value
	if(shn->last != TOKEN_VALUE) return PARSE_ERR_MISSING_VALUES;
	// Displace all fixity operators until TOKEN_PARENTH or TOKEN_COMMA
	if(displace_fixity(shn, -1)) return PARSE_ERR_MISSING_VALUES;
	
	// Count number of commas until open parenthesis
	size_t arity = 1;
	struct shn_oper_s *top = shn->ops + shn->oplen - 1;
	for(; top >= shn->ops && top->type == TOKEN_COMMA; top--) arity++;
	
	// Check for parenthesis below commas
	if(top < shn->ops || top->type != TOKEN_PARENTH){
		shn->oplen = top - shn->ops + 1;
		return PARSE_ERR_PARENTH_MISMATCH;
	}
	top--;  // Remove parenthesis
	
	// Check for function below parenthesis
	if(top >= shn->ops) switch(top->type){
		// Not function, don't call
		case TOKEN_PARENTH:
		case TOKEN_COMMA:
		case TOKEN_FIXITY:
		break;
		
		case TOKEN_MCODE_FUNC:
			// If variable hasn't been initialized set arity
			mcode_set_arity(top->code, arity);
			// Check arity of code
			if(arity != mcode_get_arity(top->code)) return PARSE_ERR_ARITY_MISMATCH;
			
			// Apply user-defined function to value stack
			mcode_call_code(shn->vals, top->code);
			arity = 1;
			top--;
		break;
		case TOKEN_FUNC:
			// Check arity of function
			if(arity != top->arity) return PARSE_ERR_ARITY_MISMATCH;
			
			// Apply function to value stack
			mcode_call_func(shn->vals, top->arity, top->func, shn->try_eval);
			arity = 1;
			top--;
		break;
	}
	
	if(arity > 1) return PARSE_ERR_BAD_COMMA;  // Must have one value
	
	// Function call or parenthetical block behaves as value
	shn->last = TOKEN_VALUE;
	shn->oplen = top - shn->ops + 1;
	return PARSE_ERR_OK;
}

parse_err_t shunt_put_comma(shunt_t shn){
	// Check that last token was value
	if(shn->last != TOKEN_VALUE) return PARSE_ERR_MISSING_VALUES;
	// Displace all operators until TOKEN_PARENTH or TOKEN_COMMA
	if(displace_fixity(shn, -1)) return PARSE_ERR_MISSING_VALUES;
	
	struct shn_oper_s *op = ops_inc(shn);
	op->type = TOKEN_COMMA;
	op->priority = -1;
	op->arity = 0;
	shn->last = TOKEN_COMMA;
	return PARSE_ERR_OK;
}

parse_err_t shunt_clear(shunt_t shn){
	// Displace all remaining fixity operators
	if(displace_fixity(shn, -1)) return PARSE_ERR_MISSING_VALUES;
	
	if(shn->oplen == 0) return PARSE_ERR_OK;
	
	// Check for remaining operators
	struct shn_oper_s *top = shn->ops + shn->oplen - 1;
	switch(shn->ops[shn->oplen - 1].type){
		case TOKEN_PARENTH: return PARSE_ERR_PARENTH_MISMATCH;
		case TOKEN_COMMA: return PARSE_ERR_BAD_COMMA;
		case TOKEN_MCODE_FUNC:
		case TOKEN_FUNC: return PARSE_ERR_FUNC_NOCALL;
	}
}



// Shunt unary operator onto operator stack
parse_err_t shunt_put_unary(shunt_t shn, arith_func_t func, int prec){
	// Can't have value before unary
	if(shn->last == TOKEN_VALUE) return PARSE_ERR_MISSING_OPERS;
	// Can't have function with no arguments
	if(shn->last == TOKEN_FUNC || shn->last == TOKEN_MCODE_FUNC) return PARSE_ERR_FUNC_NOCALL;
	
	// Previous token can't be left-associative infix with higher precedence
	struct shn_oper_s top;
	if(shn->last == TOKEN_FIXITY
	&& (top = shn->ops[shn->oplen - 1]).arity == 2
	&& (top.priority & 1)
	&& top.priority > (prec << 1)
	) return PARSE_ERR_LOWPREC_UNARY;
	
	struct shn_oper_s *op = ops_inc(shn);
	op->type = TOKEN_FIXITY;
	op->priority = (prec << 1) | 1;
	op->arity = 1;
	op->func = func;
	shn->last = TOKEN_FIXITY;
	return PARSE_ERR_OK;
}

// Shunt binary operator onto operator stack
parse_err_t shunt_put_binary(shunt_t shn, arith_func_t func, int prec, bool left_assoc){
	// Previous token must be value
	if(shn->last != TOKEN_VALUE) return PARSE_ERR_MISSING_VALUES;
	// Displace all operators of higher precedence
	if(displace_fixity(shn, prec << 1)) return PARSE_ERR_MISSING_VALUES;
	
	struct shn_oper_s *op = ops_inc(shn);
	op->type = TOKEN_FIXITY;
	op->priority = (prec << 1) | !!left_assoc;
	op->arity = 2;
	op->func = func;
	shn->last = TOKEN_FIXITY;
	return PARSE_ERR_OK;
}



#define check_value_like(shn) {\
	/* Check for function with no call */\
	if((shn)->last == TOKEN_FUNC || (shn)->last == TOKEN_MCODE_FUNC) return PARSE_ERR_FUNC_NOCALL;\
	/* Previous token can't be a value */\
	if((shn)->last == TOKEN_VALUE) return PARSE_ERR_MISSING_OPERS;\
}

// Shunt builtin function call to operator stack
parse_err_t shunt_func_call(shunt_t shn, int arity, arith_func_t func){
	check_value_like(shn);
	struct shn_oper_s *op = ops_inc(shn);
	op->type = TOKEN_FUNC;
	op->priority = -1;
	op->arity = arity;
	op->func = func;
	shn->last = TOKEN_FUNC;
	return PARSE_ERR_OK;
}

// Shunt user-defined function call to operator stack
parse_err_t shunt_code_call(shunt_t shn, mcode_t callee){
	check_value_like(shn);
	// Ensure that `var` has arguments
	if(!mcode_get_arity(callee)) return PARSE_ERR_VAR_CALL;
	
	struct shn_oper_s *op = ops_inc(shn);
	op->type = TOKEN_MCODE_FUNC;
	op->priority = -1;
	op->arity = 0;  // Only used for builtin function
	op->code = callee;
	shn->last = TOKEN_MCODE_FUNC;
	return PARSE_ERR_OK;
}

// Load argument immediately into code block
parse_err_t shunt_load_arg(shunt_t shn, int arg){
	check_value_like(shn);	
	mcode_load_arg(shn->vals, arg);
	shn->last = TOKEN_VALUE;
	return PARSE_ERR_OK;
}

// Load constant immediately into code block
parse_err_t shunt_load_const(shunt_t shn, arith_t value){
	check_value_like(shn);
	mcode_load_const(shn->vals, value);
	shn->last = TOKEN_VALUE;
	return PARSE_ERR_OK;
}

// Load another code block (must have arity 0) immediately into code block
parse_err_t shunt_load_var(shunt_t shn, mcode_t var){
	check_value_like(shn);
	// Ensure that `var` has no arguments
	mcode_set_arity(var, 0);
	if(mcode_get_arity(var)) return PARSE_ERR_FUNC_NOCALL;
	
	mcode_call_code(shn->vals, var);
	shn->last = TOKEN_VALUE;
	return PARSE_ERR_OK;
}

