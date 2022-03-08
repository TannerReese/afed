#include <stdlib.h>

#include "mcode.h"

enum instr_type {
	INSTR_CONST_LOAD,
	INSTR_ARG_LOAD,
	INSTR_CODE_CALL,
	INSTR_FUNC_CALL
};

// Single instruction
struct instr_s {
	enum instr_type type : 4;
	
	/* Number of arguments in `func` call
	 * Only used when type == INSTR_FUNC_CALL
	 */
	int arity;
	
	union {
		int arg;  // Index of argument
		
		arith_func_t func;  // Pointer to function to call
		arith_t value;  // Pointer to constant
		mcode_t code;  // Pointer to other block of code
	};
};

// Sequence of instructions to execute
struct mcode_s {
	// Current number of instruction and max number
	size_t len, cap;
	// Array of instructions
	struct instr_s *instrs;
	
	// Number of arguments that this code block takes
	// Equals zero if it is constant
	// Negative if arity is undetermined
	int arity;
	// Stack Height: Number of values left on stack
	// if code were evaluated (Equal to 1 for runnable code)
	int stk_ht;
	
	// Cached return value and error
	bool is_cached : 1;
	arith_err_t err;
	arith_t value;
};



// Print out error or refer to arith_strerror
const char *mcode_strerror(arith_err_t err){
	switch(err){
		case EVAL_ERR_MISSING_ARGS: return "EVAL_ERR_MISSING_ARGS: Not enough arguments for function call";
		case EVAL_ERR_UNKNOWN_INSTR: return "EVAL_ERR_UNKNOWN_INSTR: Instruction type not recognized";
		case EVAL_ERR_STACK_SURPLUS: return "EVAL_ERR_STACK_SURPLUS: Values on Stack after Execution complete";
		case EVAL_ERR_UNDERFLOW: return "EVAL_ERR_UNDERFLOW: Too few values on stack";
		case EVAL_ERR_INCOMPLETE_CODE: return "EVAL_ERR_INCOMPLETE_CODE: Code Block doesn't have enough instructions";
	}
	
	return arith_strerror(err);
}


/*  MCode Manipulator Functions
 * =============================
 */
mcode_t mcode_new(int arity, size_t cap){
	mcode_t code = malloc(sizeof(struct mcode_s));
	code->arity = arity;
	code->stk_ht = 0;
	
	// Allocate instruction vector
	code->len = 0;  code->cap = cap;
	code->instrs = malloc(cap * sizeof(struct instr_s));
	
	// Cache is initially empty
	code->is_cached = false;
	code->err = EVAL_ERR_OK;
	return code;
}

void mcode_free(mcode_t code){
	// Free any values associated with instructions
	for(size_t i = 0; i < code->len; i++){
		struct instr_s instr = code->instrs[i];
		if(instr.type == INSTR_CONST_LOAD) arith_free(instr.value);
	}
	
	free(code->instrs);  // Free instruction vector
	if(code->is_cached) arith_free(code->value);  // Free any cached value
	free(code);
}



mcode_t *mcode_deplist(mcode_t code, size_t *lenp){
	size_t len = 0, cap = 4;
	mcode_t *deps = malloc(cap * sizeof(mcode_t));  // Allocate array
	
	// Collect dependencies
	struct instr_s *instr, *end_instr = code->instrs + code->len;
	for(instr = code->instrs; instr < end_instr; instr++) if(instr->type == INSTR_CODE_CALL){
		mcode_t callee = instr->code;
		size_t i;
		for(i = 0; i < len; i++){  // Check for duplicates
			if(callee == deps[i]) break;
		}
		if(i < len) continue;  // When duplicate found
		
		// Otherwise add to list
		if(len >= cap){
			cap <<= 1;
			deps = realloc(deps, cap * sizeof(mcode_t));
		}
		deps[len++] = callee;
	}
	
	// Remove unneeded space
	deps = realloc(deps, len * sizeof(mcode_t));
	if(lenp) *lenp = len;
	return deps;
}

// Attempt to change the arity of code block
int mcode_set_arity(mcode_t code, int new_arity){
	if(code->arity >= 0) return -1;
	
	// Check instructions for maximum argument index
	int max_arg = -1;
	for(size_t i = 0; i < code->len; i++){
		struct instr_s instr = code->instrs[i];
		if(instr.type == INSTR_ARG_LOAD && instr.arg > max_arg)
			max_arg = instr.arg;
	}
	// New arity is not big enough
	if(max_arg >= new_arity) return max_arg + 1;
	
	code->arity = new_arity;  // Set new arity
	return 0;
}

// Get current arity
int mcode_get_arity(mcode_t code){
	return code->arity;
}

// Return the stack height of the code block
int mcode_stack_height(mcode_t code){
	return code->stk_ht;
}

// Clear any cached values
bool mcode_clear(mcode_t code){
	if(!code->is_cached) return false;
	
	code->is_cached = false;
	code->err = EVAL_ERR_OK;
	arith_free(code->value);
	return true;
}

// Remove all instructions and caching from code block
void mcode_reset(mcode_t code){
	mcode_clear(code);
	code->stk_ht = 0;
	code->len = 0;
}

// Return cached error if present
arith_err_t mcode_error(mcode_t code){
	return code->err;
}



static inline struct instr_s *instrs_inc(mcode_t code){
	if(code->len >= code->cap){  // Resize if necessary
		code->cap += code->cap == 0;  // Make sure capacity is at least one
		code->cap <<= 1;
		code->instrs = realloc(code->instrs, code->cap * sizeof(struct instr_s));
	}
	return code->instrs + (code->len++);  // Return pointer to new element
}

bool mcode_load_const(mcode_t code, arith_t value){
	if(!code || code->stk_ht < 0) return true;  // Value can't be NULL
	mcode_clear(code);
	
	struct instr_s *instr = instrs_inc(code);  // Get pointer to new instr
	instr->type = INSTR_CONST_LOAD;
	instr->arity = 0;
	instr->value = value;  // Place value in instruction
	
	code->stk_ht++;  // Update Stack Height
	return false;
}

bool mcode_load_arg(mcode_t code, int arg){
	if(!code || code->stk_ht < 0 || arg < 0
	|| (code->arity >= 0 && code->arity < arg)
	) return true;  // Argument must be within arity (if set)
	mcode_clear(code);  // Clear any cache if present
	
	struct instr_s *instr = instrs_inc(code);  // Get pointer to new instr
	instr->type = INSTR_ARG_LOAD;
	instr->arity = 0;
	instr->arg = arg;  // Store index of argument
	
	code->stk_ht++;  // Update Stack Height
	return false;
}

bool mcode_call_code(mcode_t code, mcode_t callee){
	if(!code || code->stk_ht < 0 || !callee  // Code pointer can't be NULL
	|| callee->arity < 0  // Invalid arity
	|| code->stk_ht < callee->arity // Insufficient arguments on stack
	) return true;
	mcode_clear(code);  // Clear any cache if present
	
	struct instr_s *instr = instrs_inc(code);  // Get pointer to new instr
	instr->type = INSTR_CODE_CALL;
	instr->arity = callee->arity;  // Store arity of code block
	instr->code = callee;  // Store pointer to code block
	
	code->stk_ht -= callee->arity - 1;  // Update Stack Height
	return false;
}

bool mcode_call_func(mcode_t code, int arity, arith_func_t func, bool try_eval){
	if(!code || code->stk_ht < 0 || !func  // Function pointer can't be NULL
	|| arity < 0  // Invalid arity
	|| code->stk_ht < arity  // Insufficient arguments on stack
	) return true;
	mcode_clear(code);  // Clear any cache if present
	
	if(try_eval){  // Try to Evaluate function immediately
		arith_t args[arity];
		size_t i;
		for(i = 0; i < arity; i++){
			struct instr_s instr = code->instrs[i + code->len - arity];
			if(instr.type != INSTR_CONST_LOAD) break;  // Ensure that all arguments are constant
			args[i] = instr.value; // Collect arguments into array
		}
		
		if(i == arity){  // If every arugment was constant then evaluate
			arith_err_t err = ARITH_ERR_OK;
			arith_t ret = func(args, &err);
			
			code->len -= arity;  // Remove instructions
			code->stk_ht -= arity;  // Update Stack Height
			
			// If Error happens cache it
			if(err){
				code->is_cached = true;
				code->err = err;
				return true;
			}
			
			return mcode_load_const(code, ret);
		}
	}
	
	struct instr_s *instr = instrs_inc(code);  // Get pointer to new instr
	instr->type = INSTR_FUNC_CALL;
	instr->arity = arity;
	instr->func = func;  // Store pointer to function
	
	code->stk_ht -= arity - 1;  // Update Stack Height
	return false;
}





/*  MCode Evaluator
 * =================
 */
// Stack of pointers to values used during execution
struct stack_s {
	size_t top, cap;  // Index of top of stack and capacity
	arith_t *ptr;  // Pointer to beginning of stack
};

// Push value onto stack and return the its index
static arith_t *stk_push(struct stack_s *stk){
	if(stk->top >= stk->cap){  // Check for resize
		stk->cap <<= 1;
		stk->ptr = realloc(stk->ptr, stk->cap * sizeof(arith_t));
	}
	return stk->ptr + (stk->top++);
}



static arith_err_t mcode_eval_stk(mcode_t code, arith_t *args, struct stack_s *stk){
	if(code->is_cached){  // Check for cached value
		*stk_push(stk) = arith_clone(code->value);
		return code->err;
	}
	
	if(code->stk_ht != 1){  // Check that code block is valid
		return EVAL_ERR_INCOMPLETE_CODE;
	}
	
	size_t start = stk->top;  // Save starting position of top
	
	// Execute instructions
	arith_err_t err = EVAL_ERR_OK;
	for(size_t i = 0; i < code->len && !err; i++){
		struct instr_s instr = code->instrs[i];
		int argidx;  // Stack index of first call argument
		
		switch(instr.type){
			case INSTR_CONST_LOAD:  // Place copy of value on stack
				*stk_push(stk) = arith_clone(instr.value);
			break;
			case INSTR_ARG_LOAD:  // Place copy of argument on stack
				*stk_push(stk) = arith_clone(args[instr.arg]);
			break;
			
			case INSTR_CODE_CALL:  // Call another code section
			case INSTR_FUNC_CALL:  // Call a function
				if(instr.arity < 0){  // Check that arity is defined
					err = EVAL_ERR_INCOMPLETE_CODE;
					break;
				}
				
				argidx = (int)stk->top - instr.arity;
				if(argidx < 0){  // Check for sufficient arguments
					err = EVAL_ERR_MISSING_ARGS;
					break;
				}
				
				if(instr.type == INSTR_CODE_CALL){  // Call other code segment
					err = mcode_eval_stk(instr.code, stk->ptr + argidx, stk);
				}else{
					// Call function and place return value above arguments
					arith_t *ret = stk_push(stk);
					*ret = instr.func(stk->ptr + argidx, &err);
				}
				
				if(err) break;  // Leave on error
				stk->top--;  // Make stk->top point to return value
				
				// Move return value to beginning of where arguments were
				stk->ptr[argidx] = stk->ptr[stk->top];
				stk->top = argidx + 1;  // Move stack top to just above argidx
			break;
			
			default: err = EVAL_ERR_UNKNOWN_INSTR;
		}
	}
	
	// Check for incorrect number of arguments after execution
	if(!err){
		if(stk->top > start + 1) err = EVAL_ERR_STACK_SURPLUS;
		else if(stk->top <= start) err = EVAL_ERR_UNDERFLOW;
	}
	
	// If code takes no arguments
	// Cache value for future use
	if(code->arity == 0){
		code->is_cached = true;
		code->err = err;
		if(!err) code->value = arith_clone(stk->ptr[stk->top - 1]);
	}
	
	return err;
}

arith_t mcode_eval(mcode_t code, arith_t *args, arith_err_t *errp){
	if(code->is_cached){  // Check for cached value
		if(errp) *errp = code->err;
		return arith_clone(code->value);
	}
	
	// Create stack to store values
	struct stack_s stk;
	stk.top = 0;
	stk.cap = 8;
	stk.ptr = malloc(stk.cap * sizeof(arith_t));
	
	// Evaluate code with stack
	arith_err_t err = mcode_eval_stk(code, args, &stk);
	if(errp) *errp = err;
	
	// Get return value
	arith_t ret = stk.ptr[0];
	// Deallocate any values remaining on stack
	for(size_t i = 1; i < stk.top; i++) arith_free(stk.ptr[i]);
	// Deallocate stack
	free(stk.ptr);
	return ret;
}

