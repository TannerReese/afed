#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>

#include "expr.h"


// Returns a string containing a description of errors
const char *expr_strerror(expr_err_t err){
	switch(err){	
		case EXPR_ERR_OK: return "EXPR_ERR_OK: Successful";
		
		// Evaluation Errors
		case EVAL_ERR_STACK_OVERFLOW: return "EVAL_ERR_STACK_OVERFLOW: Values pushed when Stack full";
		case EVAL_ERR_STACK_UNDERFLOW: return "EVAL_ERR_STACK_UNDERFLOW: Values popped when Stack empty";
		case EVAL_ERR_STACK_SURPLUS: return "EVAL_ERR_STACK_SURPLUS: Too Many values on stack at end of program";
		case EVAL_ERR_NO_EXPR: return "EVAL_ERR_NO_EXPR: Referenced Variable didn't have expression";
		
		// Parsing Errors
		case PARSE_ERR_PARENTH_MISMATCH: return "PARSE_ERR_PARENTH_MISMATCH: Missing open or close parenthesis";
		case PARSE_ERR_LOWPREC_UNARY: return "PARSE_ERR_LOWPREC_UNARY: Unary operator follows Binary of Higher Precedence";
		case PARSE_ERR_TOO_MANY_VALUES: return "PARSE_ERR_TOO_MANY_VALUES: Expression tree is too deep";
		case PARSE_ERR_MISSING_VALUES: return "PARSE_ERR_MISSING_VALUES: Operator is missing argument";
		case PARSE_ERR_MISSING_OPERS: return "PARSE_ERR_MISSING_OPERS: Multiple values without operator between";
		case PARSE_ERR_EXTRA_CONT: return "PARSE_ERR_EXTRA_CONT: Values present after expression";
		
		// Insertion Errors
		case INSERT_ERR_REDEF: return "INSERT_ERR_REDEF: Variable already exists";
		case INSERT_ERR_CIRC: return "INSERT_ERR_CIRC: Variable depends on itself";
	}
	
	if(err > 0){
		if(expr_arith_strerror){
			const char *str = expr_arith_strerror(err);
			if(str) return str;
		}
		
		// Default message for arithmetic errors
		return "EVAL_ERR_ARITH: Arithmetic Error";
	}
	
	// Unknown error
	return NULL;
}




typedef uint16_t instr_t;

#define INSTR_IS_OPER(inst) ((inst) & 0x8000)
#define INSTR_IS_BINARY(inst) (((inst) & 0xc000) == 0x8000)
#define INSTR_IS_UNARY(inst) (((inst) & 0xc000) == 0xc000)
#define INSTR_OPERID(inst) ((inst) & 0x3fff)
// Create operator instruction using oper_t
#define INSTR_NEW_OPER(op, unary) (0x8000 | (!!(unary) << 14) | ((op) & 0x3fff))

#define INSTR_IS_VAR(inst) (((inst) & 0xc000) == 0x4000)
#define INSTR_IS_CONST(inst) (!((inst) & 0xc000))
#define INSTR_LOAD_INDEX(inst) ((inst) & 0x3fff)
// Create variable load instruction using index
#define INSTR_NEW_VAR(idx) (0x4000 | ((idx) & 0x3fff))
// Create constant load instruction using index
#define INSTR_NEW_CONST(idx) ((idx) & 0x3fff)

// Get constant or variable from expression
#define INSTR_LOAD(exp, inst) (INSTR_IS_VAR(inst) ? (exp)->vars[INSTR_LOAD_INDEX(inst)]->cached : get_const((exp), INSTR_LOAD_INDEX(inst)))

struct expr_s {
	// Outside variables loaded at runtime
	size_t varlen, varcap;
	var_t *vars;
	
	// Constants & Literals
	size_t constlen, constcap;  // Number of members and maximum number possible
	// Block of memory containing values
	// Each value is `expr_valctl.size` bytes in size
	void *consts;
	
	// Instructions to Run
	size_t instrlen, instrcap;
	instr_t *instrs;
};

// Access constant value at index i
#define get_const(exp, i) ((exp)->consts + (i) * expr_valctl.size)
#define set_const(exp, i, val) valmove(get_const(exp, i), val)


typedef uint32_t hash_t;

struct var_s {
	// Expression used to calculate the value of this variable
	expr_t expr;
	
	// Cached value of calculation
	bool is_cached : 1;  // Indicate if a value is stored in cached
	void *cached;
	// Error that occurred when calculating cached
	expr_err_t err;
	
	// Name of variable
	size_t namelen;
	const char *name;
	hash_t hash;  // 32-bit hash of name
	
	// Next sibling in the linked list
	struct var_s *next;
	
	/* When checking dependencies for variable x
	 * This stores the variable through which x relies on this one
	 * Thus following the used_by field forms a linked list back to x
	 */
	struct var_s *used_by;
};

static hash_t hash(const char *str, size_t len){
	hash_t c, h = 0x9bcb43f7;
	for(; len > 0; len--) h = ((h << 5) + h) ^ *(str++);
	return h;
}

// Get name for variable
const char *nmsp_var_name(var_t vr, size_t *len){
	if(len) *len = vr->namelen;
	return vr->name;
}

// Get expression for variable
expr_t nmsp_var_expr(var_t vr){
	return vr->expr;
}

// Get value of variable and place into `dest`
expr_err_t nmsp_var_value(void *dest, var_t vr){
	// Allocate space for variable if not present
	if(!vr->cached) vr->cached = malloc(expr_valctl.size);
	// Calculate the value if not cached
	if(!vr->is_cached) vr->err = expr_eval(vr->cached, vr->expr);
	
	valmove(dest, vr->cached);
	return vr->err;
}

// Print variable value to a file
int nmsp_var_fprint(FILE *stream, var_t vr){
	// Get value
	if(!vr->cached) vr->cached = malloc(expr_valctl.size);
	if(!vr->is_cached) vr->err = expr_eval(vr->cached, vr->expr);
	
	if(vr->err) return fprintf(stream, "ERR %i", vr->err);
	else return expr_valctl.print(stream, vr->cached);
}



struct namespace_s {
	// Head of Linked List of variables
	struct var_s *head;
	
	/* On Insertion Error due to Redefinition
	 *  Store the variable that was attempted to be redefined
	 */
	var_t redef;
	/* Used by dependency checker
	 *  `circ_root` is a variable which depends
	 *  on itself through a series of variables
	 */
	var_t circ_root;
};

// Create new empty namespace
namespace_t nmsp_new(size_t cap){
	namespace_t nmsp = malloc(sizeof(struct namespace_s));
	nmsp->head = NULL;
	nmsp->redef = NULL;
	nmsp->circ_root = NULL;
	return nmsp;
}

// Deallocate namespace, its variables, and their expressions
void nmsp_free(namespace_t nmsp){
	// Free variables of namespace
	var_t vr, next = nmsp->head;
	while(next){
		vr = next;
		
		// Free any expression the variable might have
		if(vr->expr) expr_free(vr->expr);
		
		// Free cached value if present
		if(vr->cached){
			// Free any outside memory this value holds
			if(vr->is_cached) valfree(vr->cached);
			// Free the actual storage space this value is in
			free(vr->cached);
		}
		
		// Get pointer to next variable
		next = vr->next;
		free(vr);
	}
	
	// Deallocate namespace itself
	free(nmsp);
}

// Get instance of variable using name
var_t nmsp_get(namespace_t nmsp, const char *key, size_t keylen){
	// Empty key is not searchable
	if(!key || keylen == 0) return NULL;
	
	hash_t keyhash = hash(key, keylen);
	for(var_t vr = nmsp->head; vr; vr = vr->next){
		if(vr->hash == keyhash  // Check for matching hash (should filter out most time)
		&& vr->namelen == keylen  // Check for same length
		&& strncmp(vr->name, key, keylen) == 0)  // Finally perform string comparison
			return vr;
	}
	return NULL;
}



// Place new variable in namespace
// WARNING: Does not perform any checks for existence or dependency
static var_t place_var_unsafe(namespace_t nmsp, const char *key, size_t keylen, expr_t exp){
	var_t vr = malloc(sizeof(struct var_s));
	// Set Expression with no cached value yet
	vr->expr = exp;
	vr->is_cached = false;
	vr->cached = NULL;
	vr->err = EXPR_ERR_OK;
	
	// Store name of variable
	vr->name = key;
	vr->namelen = keylen;
	vr->hash = hash(key, keylen);
	
	// Will be used during nmsp_insert
	vr->used_by = NULL;
	
	// Place variable at head of linked list
	vr->next = nmsp->head;
	nmsp->head = vr;
	
	// Return pointer to variable
	return vr;
}

// Create variable with given name but with no expression
var_t nmsp_put(namespace_t nmsp, const char *key, size_t keylen){
	// Return if there already is a variable with that name
	if(nmsp_get(nmsp, key, keylen)) return NULL;
	return place_var_unsafe(nmsp, key, keylen, NULL);
}


// Queue methods used by nmsp_insert
// ----------------------------------
struct queue_s {
	// Pointer to memory allocated for queue
	var_t *ptr;
	
	// Offset of beginning of queue from `ptr`
	size_t start;
	// Number of elements in queue & Maximum number possible
	size_t len, cap;
};

// Create queue with given capacity and initial content
static struct queue_s queue_new(size_t cap){
	struct queue_s q;
	q.ptr = malloc(cap * sizeof(var_t));
	q.start = 0;  q.len = 0;
	q.cap = cap;
	return q;
}

// Returns the new capacity
static size_t queue_enlarge(struct queue_s *q, size_t newlen){
	if(newlen > q->cap){
		size_t oldcap = q->cap;
		
		while(newlen > q->cap) q->cap <<= 1;
		// Allocate new capacity
		q->ptr = realloc(q->ptr, sizeof(var_t) * q->cap);
		
		if(q->start + q->len > oldcap){
			// Move any discontiguous piece together
			memcpy(q->ptr + oldcap, q->ptr, (q->start + q->len - oldcap) * sizeof(var_t));
		}

	}
	return q->cap;
}

// Add element to the end of the queue
static void queue_push(struct queue_s *q, var_t *vars, size_t varlen){
	// Make sure queue is long enough
	queue_enlarge(q, q->len + varlen);
	
	// Find end of queue
	size_t end = q->start + q->len;
	end -= (end >= q->cap) * q->cap;
	
	// Increase number of elements
	q->len += varlen;
	// Copy elements
	int extra = (int)(end + varlen) - q->cap;
	if(extra > 0){
		memcpy(q->ptr + end, vars, sizeof(var_t) * (varlen - extra));
		memcpy(q->ptr, vars + extra, sizeof(var_t) * extra);
	}else{
		memcpy(q->ptr + end, vars, sizeof(var_t) * varlen);
	}
}

// Remove element from the beginning of the queue
static var_t queue_pop(struct queue_s *q){
	// Return NULL if no values in the queue
	if(q->len == 0) return NULL;
	
	var_t vr = q->ptr[q->start];
	q->start++;
	q->start -= (q->start >= q->cap) * q->cap;
	q->len--;
	return vr;
}

/* Try to insert an expression with the given name
 * If the expression already exists or has a circular dependency returns NULL
 * If there is a circular dependency the dependency list will be set
 */
var_t nmsp_insert(namespace_t nmsp, const char *key, size_t keylen, expr_t exp){
	// Check for a forward declaration for this expression
	var_t newvr;
	if(newvr = nmsp_get(nmsp, key, keylen)){
		if(newvr->expr){
			// Check if variable already has an expression
			// Error: key is already defined
			nmsp->redef = newvr;
			nmsp->circ_root = NULL;
			return NULL;
		}
		nmsp->redef = NULL;  // No redefinition
		
		
		/* Check for circular dependency
		 * Only necessary if there was a forward declaration
		 */
		
		// Clear out any previous dependency tree
		nmsp->circ_root = NULL;
		for(var_t v = nmsp->head; v; v = v->next) v->used_by = NULL;
		
		// Initialize with exp's immediate dependencies
		struct queue_s q = queue_new(exp->varlen << 1);
		queue_push(&q, exp->vars, exp->varlen);
		// Set their reference to `newvr`
		for(size_t i = 0; i < exp->varlen; i++) exp->vars[i]->used_by = newvr;
		
		// Iterate over variables checking their dependencies
		while(q.len > 0){  // While there are remaining variables to check
			// Get variable
			var_t vr = queue_pop(&q);
			
			// Check if it matches the root variable
			if(newvr == vr){
				nmsp->circ_root = vr;
				free(q.ptr);  // Cleanup queue
				return NULL;
			}
			
			// If variable's expression has no variables go to next
			if(!vr->expr || vr->expr->varlen == 0) continue;
			
			var_t *deps = vr->expr->vars;
			size_t deplen = vr->expr->varlen;
			// Add all variables used by `vr` to the queue
			queue_push(&q, deps, deplen);
			// If `used_by` is not already set
			// Set the `used_by` pointer to point to the parent node in the dependency tree
			for(size_t i = 0; i < deplen; i++) if(!deps[i]->used_by) deps[i]->used_by = vr;
		}
		
		// Free queue
		free(q.ptr);
		
		// If no circular dependency
		newvr->expr = exp;  // Set expression
		return newvr;
		
	}else{  // If no forward declared variable then create new variable
		return place_var_unsafe(nmsp, key, keylen, exp);
	}
}

var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, expr_err_t *err){
	// Parse Label
	// ------------
	// Collect label characters
	while(isblank(*str)) str++;  // Skip whitespace before
	const char *lbl = str;
	while(isalnum(*str) || *str == '_') str++;
	size_t lbl_len = str - lbl;
	while(isblank(*str)) str++;  // Skip whitespace after
	
	// Check for colon after label
	if(*str != ':' || lbl_len == 0){
		// No colon --> No label
		str = lbl;  // Move string pointer back to beginning
		lbl = NULL;  lbl_len = 0;
	}else str++;
	
	// Parse Expression
	// -----------------
	expr_t exp = expr_parse(str, endptr, nmsp, err);
	if(!exp || (err && *err)){  // On Parse Error
		return NULL;
	}
	
	// Insert Expression
	var_t vr = nmsp_insert(nmsp, lbl, lbl_len, exp);
	if(vr) return vr;
	else{
		if(err) *err = nmsp->circ_root ? INSERT_ERR_CIRC : INSERT_ERR_REDEF;
		return NULL;
	}
}



// Returns the number of characters placed into buf not including the null-byte
int nmsp_strcirc(namespace_t nmsp, char *buf, size_t sz){
	if(sz == 0) return -1;
	int count = 0;
	
	// Iterate dependency chain to produce string
	var_t crc = nmsp->circ_root;
	int isfirst = 1;
	do{
		size_t len = crc->namelen;
		count += snprintf(buf + count, sz - count, isfirst ? "%.*s" : " -> %.*s", crc->namelen, crc->name);
		isfirst = 0;
		
		crc = crc->used_by;
	}while(crc != nmsp->circ_root && count < sz);
	
	return count;
}

// Returns the number of characters placed into buf not including the null-byte
int nmsp_strredef(namespace_t nmsp, char *buf, size_t sz){
	// Get pointer to redefined variable
	var_t rdf = nmsp->redef;
	if(rdf->namelen + 1 < sz) sz = rdf->namelen + 1;
	
	sz--;  // Retain space for null-byte
	strncpy(buf, rdf->name, sz);
	buf[sz] = '\0';
	return sz;
}






// Allocate empty expression
expr_t expr_new(size_t varcap, size_t constcap, size_t instrcap){
	expr_t exp = malloc(sizeof(struct expr_s));
	exp->varlen = 0;  exp->varcap = varcap;
	exp->vars = malloc(varcap * sizeof(var_t));
	exp->constlen = 0;  exp->constcap = constcap;
	exp->consts = malloc(constcap * expr_valctl.size);
	exp->instrlen = 0;  exp->instrcap = instrcap;
	exp->instrs = malloc(instrcap * sizeof(instr_t));
	return exp;
}

// Deallocate expression
void expr_free(expr_t exp){
	free(exp->instrs);
	
	// Deallocate any stored constants
	for(size_t i = 0; i < exp->constlen; i++) valfree(get_const(exp, i));
	free(exp->consts);
	
	free(exp->vars);
	free(exp);
}

expr_t expr_new_var(var_t vr){
	expr_t exp = expr_new(2, 0, 2);
	exp->varlen = 1;  exp->vars[0] = vr;
	exp->instrlen = 1;  exp->instrs[0] = INSTR_NEW_VAR(0);
	return exp;
}

expr_t expr_new_const(void *val){
	expr_t exp = expr_new(0, 2, 2);
	exp->constlen = 1;  set_const(exp, 0, val);
	exp->instrlen = 1;  exp->instrs[0] = INSTR_NEW_CONST(0);
	return exp;
}

// Evaluate `exp` using the provided stack
static expr_err_t expr_eval_stk(expr_t exp, void *stack, void *stkend);

expr_err_t expr_eval(void *dest, expr_t exp){
	size_t stksize = EXPR_EVAL_STACK_SIZE * expr_valctl.size;
	uint8_t stack[stksize];  // Allocate Stack for evaluation
	// On evaluation error return
	expr_err_t err;
	if(!(err = expr_eval_stk(exp, stack, stack + stksize))){
		// On succes, place bottom of stack into dest
		valmove(dest, stack);
	}
	
	return err;
}

// Evaluate `exp` using the provided stack
static expr_err_t expr_eval_stk(expr_t exp, void *stack, void *stkend){
	// If no expression is provided
	if(!exp) return EVAL_ERR_NO_EXPR;
	
	// Evaluate any variable dependencies
	for(size_t i = 0; i < exp->varlen; i++){
		var_t vr = exp->vars[i];
		
		// Allocate space for variable if needed
		if(!vr->cached) vr->cached = malloc(expr_valctl.size);
		// Evaluate expression if no cached variable
		if(!vr->is_cached){
			vr->err = expr_eval_stk(vr->expr, stack, stkend);
			valmove(vr->cached, stack);
		}
		
		if(vr->err) return vr->err;
	}
	
	// Store location on stack
	void* stktop = stack;
	// Macro to shift the stack up or down
	#define shfstk(pt, shf) ((void*)((uint8_t*)(pt) + (shf) * expr_valctl.size))
	
	expr_err_t err = EXPR_ERR_OK;  // Track any errors that happen
	
	// Run Instructions
	bool no_free = true;  // If the top element doesn't need to be freed on binary operation
	for(size_t i = 0; i < exp->instrlen; i++){
		instr_t inst = exp->instrs[i];
		
		if(INSTR_IS_OPER(inst)){  // Operator
			// Check that there are enough arguments
			if(shfstk(stktop, -!INSTR_IS_UNARY(inst)) <= stack){
				err = EVAL_ERR_STACK_UNDERFLOW;
				break;
			}
			
			// Get top argument and get operator id
			void *arg = shfstk(stktop, -1);
			oper_t op = INSTR_OPERID(inst);
			
			if(INSTR_IS_UNARY(inst)){  // Unary Operator
				// Perform Unary Operation
				if(err = expr_opers[op].func.unary(arg)) break;
			}else{  // Binary Operator
				// Perform Binary Operation
				if(err = expr_opers[op].func.binary(shfstk(arg, -1), arg)) break;
				
				if(no_free) no_free = false;  // Clear no_free flag
				else valfree(arg);  // Deallocate top element
				
				stktop = arg;  // Pull stack down to reflect consumption of top element
			}
			
		}else{  // Constant or Variable load
			// Check for space on stack
			if(stktop >= stkend){
				err = EVAL_ERR_STACK_OVERFLOW;
				break;
			}
			
			// Pointer to loaded value
			void *val = INSTR_LOAD(exp, inst);
			
			// Check if the next instruction is a binary operation
			if(i < exp->instrlen - 1 && INSTR_IS_BINARY(inst = exp->instrs[i + 1])){
				// If the next instruction is a binary operation
				// Don't perform a needless allocation
				no_free = true;
				valmove(stktop, val);
			}else{
				// Otherwise a clone is necessary
				valclone(stktop, val);
			}
			
			// Move `stktop` up because of additional value
			stktop = shfstk(stktop, 1);
		}
	}
	
	// Should only be one remaining value after evaluation
	if(!err && shfstk(stktop, -1) > stack){
		err = EVAL_ERR_STACK_SURPLUS;
	}
	
	if(err){
		// Cleanup values on stack
		if(no_free) stktop = shfstk(stktop, -1);  // Ignore top value if no_free true
		for(; stktop >= stack; stktop = shfstk(stktop, -1)) valfree(stktop);
		return err;
	}else{
		// There will be one value left at the bottom of the stack
		return EXPR_ERR_OK;
	}
}



// Macro to increase array size enough for one element
// Does not modify the length only the capacity
#define inc_size(arr, size, cap, len) if((len) >= (cap)){ \
	/* Make sure capacity is at least one */ (cap) += (cap) == 0; \
	do{ (cap) <<= 1; }while((len) >= (cap)); \
	(arr) = realloc((arr), (cap) * (size)); \
}

// Place variable in expr variable section if not alreayd present
// Return the variable index
static size_t expr_put_var(expr_t exp, var_t vr){
	// Check if variable already present
	for(size_t i = 0; i < exp->varlen; i++) if(exp->vars[i] == vr) return i;
	
	// Add variable to variable list
	// Resize variable section if necessary
	inc_size(exp->vars, sizeof(var_t), exp->varcap, exp->varlen);
	exp->vars[exp->varlen++] = vr;
	return exp->varlen - 1;
}

// Place constant value in expr constants section if not already present
// Return the constant index
static size_t expr_put_const(expr_t exp, void *val){
	// Check if constant value is already present
	for(size_t i = 0; i < exp->constlen; i++){
		if(valequal(get_const(exp, i), val)){
			// If `val` is not placed into the constant list
			// Then it must be deallocated
			valfree(val);
			
			return i;
		}
	}
	
	// Add value to constants list
	// Resize constant section if necessary
	inc_size(exp->consts, expr_valctl.size, exp->constcap, exp->constlen);
	set_const(exp, exp->constlen, val);
	exp->constlen++;
	return exp->constlen - 1;
}

/* Remove the last instruction if it is a const load
 * Additionally remove the corresponding const if no other loads use it
 *
 * Returns 1 if constant was removed and 0 otherwise
 * If the constant was removed the value is placed in `dest`
 * Otherwise the value of the constant is cloned into `dest`
 */

static bool expr_pop_const_load(expr_t exp, void *dest){
	instr_t instr = exp->instrs[exp->instrlen - 1];  // Get top instruction
	
	// Make sure top instruction is a constant load
	if(!INSTR_IS_CONST(instr)) return 0;
	// Remove top instruction
	exp->instrlen--;
	
	// Get associated constant
	size_t idx = INSTR_LOAD_INDEX(instr);
	void *val = get_const(exp, idx);
	
	// Check if any other instructions use this constant
	for(size_t i = 0; i < exp->instrlen; i++){
		instr = exp->instrs[i];
		// If any other instruction uses this constant
		// We must clone the value into `dest`
		if(INSTR_IS_CONST(instr) && INSTR_LOAD_INDEX(instr) == idx){
			valclone(dest, val);
			return false;
		}
	}
	
	// If no other instructions use this constant remove it
	exp->constlen--;
	// Place the value into `dest`
	valmove(dest, val);
	return true;
}


// Combine `vr` onto expression `exp` using binary operator `op`
expr_t expr_binary_var(expr_t exp, var_t vr, oper_t op){
	// Put variable into expression
	size_t varid = expr_put_var(exp, vr);
	
	// Resize if necessary to hold two new instructions
	exp->instrlen++;
	inc_size(exp->instrs, sizeof(instr_t), exp->instrcap, exp->instrlen);
	
	// Add load instruction to instruction list
	exp->instrs[exp->instrlen - 1] = INSTR_NEW_VAR(varid);
	// Add operator instruction
	exp->instrs[exp->instrlen++] = INSTR_NEW_OPER(op, 0);
	return exp;
}

// Combine `val` onto expression `exp` using binary operator `op`
expr_t expr_binary_const(expr_t exp, void *val, oper_t op){
	// Put constant into expression
	size_t constid = expr_put_const(exp, val);
	
	// Add load instruction to instruction list
	exp->instrlen++;
	inc_size(exp->instrs, sizeof(instr_t), exp->instrcap, exp->instrlen);
	
	// Add load instruction to instruction list
	exp->instrs[exp->instrlen - 1] = INSTR_NEW_CONST(constid);
	// Add operator instruction
	exp->instrs[exp->instrlen++] = INSTR_NEW_OPER(op, 0);
	return exp;
}

// Combine `src` expression onto `dest` using binary operator `op`
expr_t expr_binary(expr_t dest, expr_t src, oper_t op){
	// Create mapping from old src variables to new values
	size_t varmap[src->varlen];
	// Put src variable into dest and record locations
	for(size_t i = 0; i < src->varlen; i++) varmap[i] = expr_put_var(dest, src->vars[i]);
	
	// Create mapping from old src constant to new constant locations
	size_t constmap[src->constlen];
	// Put src constant into dest and record locations
	for(size_t i = 0; i < src->constlen; i++) constmap[i] = expr_put_const(dest, get_const(src, i));
	
	
	// Iterate over src instructions to substitute variable and constant references
	for(size_t i = 0; i < src->instrlen; i++){
		instr_t inst = src->instrs[i];
		
		// Replace load index of var or const load instruction
		if(INSTR_IS_VAR(inst))
			inst = INSTR_NEW_VAR(varmap[INSTR_LOAD_INDEX(inst)]);
		else if(INSTR_IS_CONST(inst))
			inst = INSTR_NEW_CONST(constmap[INSTR_LOAD_INDEX(inst)]);
		
		inc_size(dest->instrs, sizeof(instr_t), dest->instrcap, dest->instrlen);  // Resize array if necessary
		dest->instrs[dest->instrlen++] = inst;  // Put instruction into dest instrs
	}
	
	// Place operator at end of instructions
	inc_size(dest->instrs, sizeof(instr_t), dest->instrcap, dest->instrlen);  // Resize array if necessary
	dest->instrs[dest->instrlen++] = INSTR_NEW_OPER(op, 0);
	
	return dest;
}

// Modify `exp` by applying unary operator `op`
expr_t expr_unary(expr_t exp, oper_t op){
	// Resize instruction array if necessary
	inc_size(exp->instrs, sizeof(instr_t), exp->instrcap, exp->instrlen);
	
	// Place unary operator at the end
	exp->instrs[exp->instrlen++] = INSTR_NEW_OPER(op, 1);
	return exp;
}

// Check validity by counting how the stack would move if evaluated
// Returns the error that would occur
static expr_err_t check_valid(expr_t exp){
	size_t height = 0;  // Track height of stack
	for(size_t i = 0; i < exp->instrlen; i++){
		instr_t inst = exp->instrs[i];
		if(INSTR_IS_OPER(inst)){
			// Check that there are sufficient operators
			if(height < 1 + !INSTR_IS_UNARY(inst)) return EVAL_ERR_STACK_UNDERFLOW;
			
			// Binary operator remove one element
			// Unary operator removes none
			height -= !INSTR_IS_UNARY(inst);
		}else{
			// Load Instructions add one element
			height++;
			
			// Check for overflow
			if(height > EXPR_EVAL_STACK_SIZE) return EVAL_ERR_STACK_OVERFLOW;
		}
	}
	
	// No return value
	if(height == 0) return EVAL_ERR_STACK_UNDERFLOW;
	// More than one return value
	if(height > 1) return EVAL_ERR_STACK_SURPLUS;
	
	return EXPR_ERR_OK;
}






#define isoper(c) ( \
		(c) == '!' || \
		(c) == '$' || \
		(c) == '%' || \
		(c) == '&' || \
		(c) == '*' || \
		(c) == '+' || \
		(c) == '-' || \
		(c) == '/' || \
		(c) == '<' || \
		(c) == '=' || \
		(c) == '>' || \
		(c) == '?' || \
		(c) == '@' || \
		(c) == '^' || \
		(c) == '~')

/* Struct to represent operator while on the operator stack
 */
struct stk_oper_s {
	// If true then this operator stack element represents an open parenthesis
	// It cannot be displaced by normal operators
	uint8_t is_block : 1;
	
	// Stores Precedence in higher 7 bits and
	// Associativity in the least significant bit
	uint8_t prec_assoc;
	
	// Id of operator
	oper_t operid;
};

// Operator stack
typedef struct {
	// Pointer to bottom of the stack
	struct stk_oper_s *bottom;
	
	// cap: Maximum number of elements allocated for on the stack
	// len: Number of elements on the stack
	size_t cap, len;
} oper_stack_t;

// Takes operator stack and returns top element of stack
#define stktop(opstk) ((opstk).bottom[(opstk).len - 1])


// Structure used to identify parse operators
struct oper_tree_s {
	// Character to compare against current value in string
	char value;
	// Id of operator or OPER_NULL if this node isn't an operator
	oper_t operid;
	
	// Pointer to next sibling node
	struct oper_tree_s *next;
	// Pointer to first child
	struct oper_tree_s *child;
};

/* Tries to parse an operator from the first part of `str`
 * Finds longest prefix which matches to a valid operator
 * On Success, a valid operator id is returned
 * Otherwise OPER_NULL is returned
 */
static oper_t parse_oper(const char *str, const char **endptr, bool is_unary){
	// Root of operator trees
	static struct oper_tree_s *binary_tree = NULL, *unary_tree = NULL;
	
	// Select tree to use
	struct oper_tree_s **root = is_unary ? &unary_tree : &binary_tree;
	// Construct operator tree if it doesn't exist for given type
	if(!*root){
		// Add each operator to tree
		for(oper_t opid = 0; expr_opers[opid].name; opid++){
			// Only add operators of specified type
			if(expr_opers[opid].is_unary != is_unary) continue;
			
			// Iterate through name adding nodes into tree
			const char *name = expr_opers[opid].name;
			const char *end = name + expr_opers[opid].namelen;
			
			struct oper_tree_s **node = root;  // Pointer to next node location
			for(; name < end; name++){
				// Iterate through sibling list to find node matching current character
				for(; *node; node = &((*node)->next)){
					// When character matches leave
					if(*name == (*node)->value) break;
				}
				
				// If no node matches create a new one
				if(!*node){
					*node = malloc(sizeof(struct oper_tree_s));
					(*node)->value = *name;
					(*node)->operid = OPER_NULL;
					(*node)->next = NULL;
					(*node)->child = NULL;
					
					// If this is the last character
					// identify the node with this operator
					if(name == end - 1) (*node)->operid = opid;
				}
				
				// After each loop iteration `node` will point to the child pointer of
				// the node corresponding to the character `*name`
				node = &((*node)->child);
			}
		}
	}
	
	// Set endptr to beginning for empty case
	if(endptr) *endptr = str;
	
	// Use operator tree to identify string
	struct oper_tree_s *node = *root;
	oper_t opid = OPER_NULL;
	for(; isoper(*str); str++){
		// Iterate through linked list of siblings to find match
		for(; node; node = node->next){
			if(node->value == *str){
				// `node` represents an operator record it
				if(node->operid != OPER_NULL){
					opid = node->operid;
					// If operator found set endptr
					if(endptr) *endptr = str + 1;
				}
				// If character matches descend to children of `node`
				node = node->child;
				break;
			}
		}
		
		// If no match found or child list is empty then leave
		if(!node) break;
	}
	
	return opid;
}

// Try to evaluate constants while parsing expressions
bool expr_eval_on_parse = true;

// Apply single operator to value stack by appending
static expr_err_t apply_oper(expr_t exp, oper_t opid){
	// If `expr_eval_on_parse` is set then try to
	// Evaluate constants using operator on the stack
	if(expr_eval_on_parse){
		bool is_unary = expr_opers[opid].is_unary;
		
		// Check for sufficient arguments
		if(exp->instrlen < 1 + !is_unary) return EVAL_ERR_STACK_UNDERFLOW;
		instr_t *inst = exp->instrs + exp->instrlen - 1;
		
		// Unary Operators
		expr_err_t err;
		if(is_unary){
			if(INSTR_IS_CONST(*inst)){
				// Pop constant off of instruction array
				valdef(dest);
				expr_pop_const_load(exp, dest);
				
				// Try to apply unary operator
				if(err = expr_opers[opid].func.unary(dest)){
					valfree(dest);
					return err;
				}
				
				// If successful push `dest` onto consts array
				int idx = expr_put_const(exp, dest);
				// Place load instruction back on instrs section
				exp->instrs[exp->instrlen++] = INSTR_NEW_CONST(idx);
				
				return EXPR_ERR_OK;
			}
			
		// Binary Operators
		}else{
			if(INSTR_IS_CONST(*inst) && INSTR_IS_CONST(*(inst - 1))){
				// Define arguments to binary operator
				valdef(dest);
				valdef(src);
				expr_pop_const_load(exp, src);
				expr_pop_const_load(exp, dest);
				
				// Try to apply binary operator
				if(err = expr_opers[opid].func.binary(dest, src)){
					valfree(dest);
					valfree(src);
					return err;
				}
				
				// If successful push `dest` onto consts array
				int idx = expr_put_const(exp, dest);
				// We can then deallocate `src`
				valfree(src);
				// Place load instruction back on instrs section
				exp->instrs[exp->instrlen++] = INSTR_NEW_CONST(idx);
				
				return EXPR_ERR_OK;
			}
		}
	}
	
	// Put operator onto instrs section
	inc_size(exp->instrs, sizeof(instr_t), exp->instrcap, exp->instrlen);
	exp->instrs[exp->instrlen++] = INSTR_NEW_OPER(opid, expr_opers[opid].is_unary);
	return EXPR_ERR_OK;
}


// Place `op` onto `opstk` displacing operators as necessary and applying them to `exp`
static expr_err_t displace_opers(expr_t exp, oper_stack_t *opstk, int8_t prec){
	// If prec is negative displace all
	if(prec < 0) prec = 0;
	/* The bitwise-or with OPER_LEFT_ASSOC pushes up the prec_assoc of new
	 * For Left-Associative operators this doesn't do anything
	 * so they remain equal and the bottom operator is displaced
	 * For Right-Associative operators new will be greater
	 * so the lower operator won't be displaced
	 */
	else prec = (prec << 1) | OPER_LEFT_ASSOC;
	
	// Iterate down through operators on the stack
	struct stk_oper_s elem;
	for(;
		opstk->len > 0 && !(elem = stktop(*opstk)).is_block && (uint8_t)prec <= elem.prec_assoc;
		opstk->len--
	){	
		// Try to apply operator onto expression
		expr_err_t err;
		if(err = apply_oper(exp, elem.operid)) return err;
	}
	
	return EXPR_ERR_OK;
}

// Try to parse sequence of alphanumerics into variable
static var_t parse_var(const char *str, const char **endptr, namespace_t nmsp){
	// Set endptr to beginning
	if(endptr) *endptr = str;
	
	// Collect variable name characters
	const char *name = str;
	while(isalnum(*str) || *str == '_') str++;
	
	// Leave if nothing collected
	if(str == name) return NULL;
	
	var_t vr;
	if(nmsp && (
		// Query namespace for variable
		(vr = nmsp_get(nmsp, name, str - name)) ||
		// Otherwise create new variable with name
		(vr = nmsp_put(nmsp, name, str - name))
	)){
		if(endptr) *endptr = str;
		return vr;
	}else return NULL;
}

// Parses String as Expression
expr_t expr_parse(const char *str, const char **endptr, namespace_t nmsp, expr_err_t *err){
	// Initialize operator stack
	oper_stack_t opstk;
	opstk.cap = 256;  opstk.len = 0;
	opstk.bottom = malloc(opstk.cap * sizeof(struct stk_oper_s));
	
	// Initialize expression
	expr_t exp = expr_new(4, 4, 8);
	
	// Give err reference to prevent null dereferences
	expr_err_t tmperr;
	if(!err) err = &tmperr;
	*err = EXPR_ERR_OK;
	
	// Track if the last token parsed was a constant or variable
	bool was_last_val = false;
	// Track parenthesis depth to see if newlines should be consumed
	int parenth_depth = 0;
	while(*str){	
		// Skip whitespace
		while(parenth_depth > 0 ? isspace(*str) : isblank(*str)) str++;
		if(parenth_depth == 0 && *str == '\n') break;  // Leave at newline outside parenthesis
		
		int c = *str;
		const char *after_tok = str;  // Pointer to after parsed token
		
		// Check for parentheses
		if(c == '('){
			str++;  // Consume '('
			parenth_depth++;
			// Place open parenthesis on operator stack			
			inc_size(opstk.bottom, sizeof(struct stk_oper_s), opstk.cap, opstk.len);
			opstk.bottom[opstk.len++].is_block = 1;  // Indicate that it is a block
			was_last_val = false;  // New expression so there is no last value
			continue;
		}else if(c == ')'){
			str++;  // Consume ')'
			parenth_depth--;
			// Remove all operators until block
			if(*err = displace_opers(exp, &opstk, -1)) break;
			
			// Check for block (i.e. open parenthesis)
			if(opstk.len > 0 && stktop(opstk).is_block){
				// Remove block and continue parsing
				opstk.len--;
				// Treat closed parenthetical expression as value
				was_last_val = true;
				continue;
			}else{
				// No opening parenthesis so parenth mismatch
				*err = PARSE_ERR_PARENTH_MISMATCH;
				break;
			}
		}
		
		// Try to parse operator
		oper_t opid = parse_oper(str, &after_tok, !was_last_val);
		if(opid != OPER_NULL && after_tok > str){
			struct stk_oper_s elem;
			
			// Only displace operators if binary operator
			struct oper_info_s info = expr_opers[opid];
			if(was_last_val){  // If operator follows val it is binary
				if(*err = displace_opers(exp, &opstk, (int8_t)(info.prec))) break;
			}else if(!stktop(opstk).is_block){  // When unary and previous element isn't block
				// Check that previous operator is unary, left-associative, or lower precedence
				struct oper_info_s info2 = expr_opers[stktop(opstk).operid];
				if(!(info2.is_unary || info2.assoc == OPER_RIGHT_ASSOC || info2.prec < info.prec)){
					// Otherwise we have a unary operator coming after a binary operator of higher precedence
					*err = PARSE_ERR_LOWPREC_UNARY;
					break;
				}
			}
			
			// Place operator at top of stack
			inc_size(opstk.bottom, sizeof(struct stk_oper_s), opstk.cap, opstk.len);
			elem.is_block = 0;
			elem.prec_assoc = (info.prec << 1) | info.assoc | info.is_unary;
			elem.operid = opid;
			opstk.bottom[opstk.len++] = elem;
			
			was_last_val = false;  // Operator is not a value
			str = after_tok;  // Move string forward
			continue;
		}
		
		
		// Try to parse value
		uint8_t val[expr_valctl.size];
		if(expr_valctl.parse(val, str, &after_tok)){
			// Cannot have two values in a row
			if(was_last_val){
				*err = PARSE_ERR_MISSING_OPERS;
				break;
			}
			
			// Put constant into expression
			size_t constid = expr_put_const(exp, val);
			// Place value onto instruction list as constant
			inc_size(exp->instrs, sizeof(instr_t), exp->instrcap, exp->instrlen);
			exp->instrs[exp->instrlen++] = INSTR_NEW_CONST(constid);
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Try to parse variable name
		// Collect alphanumerics and _
		var_t vr;
		if(vr = parse_var(str, &after_tok, nmsp)){
			// Cannot have two values in a row
			if(was_last_val){
				*err = PARSE_ERR_MISSING_OPERS;
				break;
			}
			
			// Place variable into variable section
			size_t varid = expr_put_var(exp, vr);
			// Place value load instruction into instruction list
			inc_size(exp->instrs, sizeof(instr_t), exp->instrcap, exp->instrlen);
			exp->instrs[exp->instrlen++] = INSTR_NEW_VAR(varid);
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Failed to parse token
		break;
	}
	
	// Move endpointer to after parsed section
	if(endptr) *endptr = str;
	
	// On error cleanup and leave
	if(*err){
		// Deallocate operator stack and exp
		expr_free(exp);
		free(opstk.bottom);
		return NULL;
	}
	
	// Clear out remaining operators on the operator stack
	for(; opstk.len > 0; opstk.len--){
		struct stk_oper_s op = stktop(opstk);
		
		// Make sure no open parenths are left
		if(op.is_block){
			*err = PARSE_ERR_PARENTH_MISMATCH;
			break;
		}
		
		// Apply operator to expression
		if(*err = apply_oper(exp, op.operid)) break;
	}
	// Deallocate stack
	free(opstk.bottom);
	
	if(*err){
		// Deallocate expression on error
		expr_free(exp);
		return NULL;
	}
	
	// Check that expression will evaluate appropriately
	if(*err = check_valid(exp)){
		// Shift error code down to be more descriptive
		*err += PARSE_ERR_TOO_MANY_VALUES - EVAL_ERR_STACK_OVERFLOW;
		expr_free(exp);
		return NULL;
	}
	
	return exp;
}

