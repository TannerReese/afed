#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>

#include "vec.h"  // Utility for vector operations
#include "nmsp.h"



// Instruction macros
// -------------------
typedef uint16_t instr_t;

// Create operator instruction using bltn_t
#define INSTR_NEW_OPER(op) (0x8000 | ((op) & 0x7fff))
#define INSTR_IS_OPER(inst) ((inst) & 0x8000)
#define INSTR_OPERID(inst) ((inst) & 0x7fff)

// Create instructions to load a variable or constant
#define INSTR_NEW_VAR(idx) (0x4000 | ((idx) & 0x3fff))
#define INSTR_NEW_CONST(idx) ((idx) & 0x3fff)

#define INSTR_IS_VAR(inst) (((inst) & 0xc000) == 0x4000)
#define INSTR_IS_CONST(inst) (!((inst) & 0xc000))

// Get index that this instruction loads from
#define INSTR_LOAD_INDEX(inst) ((inst) & 0x3fff)
// Get constant, variable, or argument from expression
#define INSTR_LOAD(exp, inst) (INSTR_IS_VAR(inst) ?\
	(exp)->vars.ptr[INSTR_LOAD_INDEX(inst)]->cached :\
	get_const((exp), INSTR_LOAD_INDEX(inst))\
)



// Variable / Namespace methods
// -----------------
// Forward declaration of expression type
struct expr_s;
typedef struct expr_s *expr_t;

typedef uint32_t hash_t;

struct var_s {
	expr_t expr;  // Expression that defines this variables
	
	void *cached;  // Cached value of calculation
	bool is_cached : 1;  // Indicate if a value is stored in cached
	nmsp_err_t err;  // Error that occurred when calculating cached
	
	// Name of variable
	size_t namelen;
	const char *name;
	hash_t hash;  // 32-bit hash of name
	
	struct var_s *next;  // Next sibling in the linked list
	
	/* When checking dependencies for variable x
	 * This stores the variable through which x relies on this one
	 * Thus following the used_by field forms a linked list back to x
	 */
	struct var_s *used_by;
};


static hash_t hash(const char *str, size_t len);
static var_t place_var_unsafe(namespace_t nmsp, const char *key, size_t keylen, expr_t exp);

// Queue methods (used for dependency checking)
static struct queue_s queue_new(size_t cap);
static size_t queue_enlarge(struct queue_s *q, size_t newlen);
static void queue_push(struct queue_s *q, var_t *vars, size_t varlen);
static var_t queue_pop(struct queue_s *q);
// Return true if `start` depends on variable `target`
static bool find_circ(namespace_t nmsp, expr_t start, var_t target);



// Expression methods
// -------------------
struct expr_s {
	// Outside variables loaded at runtime
	vec_t(var_t) vars;
	
	// Constants & Literals
	// Vector's memory contains elements each of size `nmsp_valctl.size`
	vec_t(void) consts;
	
	// Instructions to Run
	vec_t(instr_t) instrs;
};

// Access constant value at index i
#define get_const(exp, i) ((exp)->consts.ptr + (i) * nmsp_valctl.size)
#define set_const(exp, i, val) valmove(get_const(exp, i), val)
// Perform pointer arithmetic with value pointer
#define valshf(ptr, shf) ((ptr) + (shf) * nmsp_valctl.size)


// Create expression with the given capacities for each section
static expr_t expr_new(size_t varcap, size_t constcap, size_t instrcap);
// Deallocates memory allocated to expression and any constants it holds
static void expr_free(expr_t exp);

typedef vec_t(void) valstk_t;  // Stack of values used during evaluation
// Evaluate expression with result stored in the first element of `stack`
static nmsp_err_t expr_eval(expr_t exp, valstk_t stack);



/* Place variable in expr variable section if not already present
 * And add load instruction to instruction section
 */
static void expr_load_var(expr_t exp, var_t vr);

/* Place variable in expr variable section if not already present
 * And add load instruction to instruction section
 */
static void expr_load_const(expr_t exp, void *val);

/* Remove the last instruction if it is a const load
 * Additionally remove the corresponding const if no other loads use it
 *
 * Returns 1 if constant was removed and 0 otherwise
 * If the constant was removed the value is placed in `dest`
 * Otherwise the value of the constant is cloned into `dest`
 */
static bool expr_pop_const_load(expr_t exp, void *dest);



// Methods and Macros involved in expression parsing

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

// Element of operator stack
struct stk_oper_s {
	// If true then this operator stack element represents an open parenthesis or comma
	// It cannot be displaced by normal operators
	uint8_t is_block : 1;
	// If this element is a comma then it will be displaced only by close parenthesis
	uint8_t is_comma : 1;
	
	// Stores Precedence in higher 7 bits and
	// Associativity in the least significant bit
	uint8_t prec_assoc;
	
	// Id of operator
	bltn_t operid;
};

typedef vec_t(struct stk_oper_s) oper_stack_t;  // Operator stack


/* Node of operator tree
 * An operator tree is used by `parse_infix_oper`
 * to identify the longest matching operator for a string
 */
struct oper_tree_s {
	// Character to compare against current value in string
	char value;
	// Id of operator or OPER_NULL if this node isn't an operator
	bltn_t operid;
	
	// Pointer to next sibling node
	struct oper_tree_s *next;
	// Pointer to first child
	struct oper_tree_s *child;
};

/* Check validity of parsed expression
 * by counting how the stack would move if evaluated
 * Returns the error that would occur
 */
static nmsp_err_t check_valid(expr_t exp);

/* Tries to parse an operator from the first part of `str`
 * Finds longest prefix which matches to a valid operator
 * On Success, a valid operator id is returned
 * Otherwise OPER_NULL is returned
 */
static bltn_t parse_infix_oper(const char *str, const char **endptr, bool is_unary);

/* Take an operator and apply it to the values in `exp`
 * Usually, this means the operator instruction is added to `exp->instrs`
 * 
 * If evaluation-on-parsing is turned on and the loaded values are constants
 * Then the operator will be immediately evaluated
 * And the result loaded onto `exp->instrs` as a constant
 */
bool nmsp_eval_on_parse = true;  // Wehther to evaluate constants while parsing
static nmsp_err_t apply_oper(expr_t exp, bltn_t opid);

/* Place operator with id `opid` on stack
 * If opid == -1 then place an open parenthesis
 * If opid == -2 then place a comma
 */
static void push_oper(oper_stack_t *opstk, int16_t opid);

/* Pops operators from the operator stack (i.e. `opstk`)
 * while they have lower precedence than `prec`
 * Each popped operator is applied to the value stack
 * (i.e. `exp->instrs`) using `apply_oper`
 * Search "Shunting Yard Algorithm" for explanation
 */
static nmsp_err_t displace_opers(expr_t exp, oper_stack_t *opstk, int8_t prec);

/* Called when a ')' is encountered
 * Removes any remaining operators
 * And checks for function calls
 */
static nmsp_err_t close_parenth(expr_t exp, oper_stack_t *opstk);

/* Read sequence of alphanumerics and '_' as a name
 * Return matching builtin function or constant if found
 * Returns OPER_NULL if none are found
 */
static bltn_t parse_builtin(const char *str, const char **endptr);

/* Read sequence of alphanumerics and '_' as a name
 * Return matching variable from `nmsp` if found
 * Returns NULL if none are found
 */
static var_t parse_var(const char *str, const char **endptr, namespace_t nmsp);

/* Primary method for parsing expression
 * Parses as much as possuble of the string
 * If `err` is not NULL then any errors are stored in it
 */
static expr_t expr_parse(const char *str, const char **endptr, namespace_t nmsp, nmsp_err_t *err);





// Returns a string containing a description of errors
const char *nmsp_strerror(nmsp_err_t err){
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
		case PARSE_ERR_ARITY_MISMATCH: return "PARSE_ERR_ARITY_MISMATCH: Wrong number of arguments given to function";
		case PARSE_ERR_BAD_COMMA: return "PARSE_ERR_BAD_COMMA: Comma in wrong location";
		case PARSE_ERR_FUNC_NOCALL: return "PARSE_ERR_FUNC_NOCALL: Function present but not called";
		
		// Produce after parsing produces invalid expression
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

// Get value of variable and place into `dest`
nmsp_err_t nmsp_var_value(void *dest, var_t vr){
	// Allocate space for variable if not present
	if(!vr->cached) vr->cached = malloc(nmsp_valctl.size);
	// Calculate the value if not cached
	if(!vr->is_cached){
		valstk_t stack;
		vecinit_sz(stack, 32, nmsp_valctl.size);
		vr->err = expr_eval(vr->expr, stack);
		valmove(vr->cached, stack.ptr);
		vecfree(stack);
	}
	
	if(dest) valmove(dest, vr->cached);
	return vr->err;
}

// Print variable value to a file
int nmsp_var_fprint(FILE *stream, var_t vr){
	nmsp_var_value(NULL, vr);  // Force calculation of value
	if(vr->err) return fprintf(stream, "ERR %i", vr->err);
	else return nmsp_valctl.print(stream, vr->cached);
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
namespace_t nmsp_new(){
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


// Find circular dependency
static bool find_circ(namespace_t nmsp, expr_t start, var_t target){
	if(!start || !target) return false;
	
	// Clear out any previous dependency tree
	nmsp->circ_root = NULL;
	for(var_t v = nmsp->head; v; v = v->next) v->used_by = NULL;
	
	// Initialize with exp's immediate dependencies
	struct queue_s q = queue_new(start->vars.len << 1);
	queue_push(&q, start->vars.ptr, start->vars.len);
	// Set their reference to `target`
	for(size_t i = 0; i < start->vars.len; i++) start->vars.ptr[i]->used_by = target;
	
	// Iterate over variables checking their dependencies
	while(q.len > 0){  // While there are remaining variables to check
		// Get variable
		var_t vr = queue_pop(&q);
		
		// Check if it matches the root variable
		if(target == vr){
			nmsp->circ_root = target;
			free(q.ptr);  // Cleanup queue
			return true;  // Circular dependency found
		}
		
		// If variable's expression has no variables go to next
		if(!vr->expr || vr->expr->vars.len == 0) continue;
		
		var_t *deps = vr->expr->vars.ptr;
		size_t deplen = vr->expr->vars.len;
		// Add all variables used by `vr` to the queue
		queue_push(&q, deps, deplen);
		// If `used_by` is not already set
		// Set the `used_by` pointer to point to the parent node in the dependency tree
		for(size_t i = 0; i < deplen; i++) if(!deps[i]->used_by) deps[i]->used_by = vr;
	}
	
	free(q.ptr);  // Free queue
	return false;  // No circular dependency
}

var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, nmsp_err_t *err){
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
	if(!exp || (err && *err)) return NULL;  // On Parse Error
	
	// Insert Expression
	// ------------------
	var_t oldvar = nmsp_get(nmsp, lbl, lbl_len);
	
	if(oldvar){
		if(oldvar->expr){  // Check for redefinition
			*err = INSERT_ERR_REDEF;
			nmsp->redef = oldvar;
			return NULL;
		}
		
		if(find_circ(nmsp, exp, oldvar)){  // Check for circular dependency
			*err = INSERT_ERR_CIRC;
			return NULL;
		}
		
		oldvar->expr = exp;  // Set variable's expression
		return oldvar;
	
	// Create new variable
	}else return place_var_unsafe(nmsp, lbl, lbl_len, exp);
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
		count += snprintf(buf + count, sz - count,
			isfirst ? "%.*s" : " <- %.*s", crc->namelen, crc->name
		);
		isfirst = 0;
		
		crc = crc->used_by;
	}while(crc != nmsp->circ_root && count < sz);
	
	// Close circle by printing root again
	count += snprintf(buf + count, sz - count, " <- %.*s", crc->namelen, crc->name);
	
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
static expr_t expr_new(size_t varcap, size_t constcap, size_t instrcap){
	expr_t exp = malloc(sizeof(struct expr_s));
	vecinit(exp->vars, varcap);
	vecinit_sz(exp->consts, constcap, nmsp_valctl.size);
	vecinit(exp->instrs, instrcap);
	return exp;
}

// Deallocate expression
static void expr_free(expr_t exp){
	vecfree(exp->instrs);
	
	// Deallocate any stored constants
	for(size_t i = 0; i < exp->consts.len; i++) valfree(get_const(exp, i));
	vecfree(exp->consts);
	
	vecfree(exp->vars);
	free(exp);
}


// Evaluate `exp` using the provided stack
static nmsp_err_t expr_eval(expr_t exp, valstk_t stack){
	// If no expression is provided
	if(!exp) return EVAL_ERR_NO_EXPR;
	
	// Evaluate any variable dependencies
	for(size_t i = 0; i < exp->vars.len; i++){
		var_t vr = exp->vars.ptr[i];
		
		// Allocate space for variable if needed
		if(!vr->cached) vr->cached = malloc(nmsp_valctl.size);
		// Evaluate expression if no cached variable
		if(!vr->is_cached){
			vr->err = expr_eval(vr->expr, stack);
			valmove(vr->cached, stack.ptr);
		}
		
		if(vr->err) return vr->err;
	}
	
	nmsp_err_t err = EXPR_ERR_OK;  // Track any errors that happen
	
	// Run Instructions
	for(size_t i = 0; i < exp->instrs.len; i++){
		instr_t inst = exp->instrs.ptr[i];
		
		if(INSTR_IS_OPER(inst)){  // Operator
			struct bltn_info_s info = nmsp_bltns[INSTR_OPERID(inst)];
			void *args = vecremove_sz(stack, info.arity, nmsp_valctl.size);
			// Check that there are enough arguments
			if(!args){ err = EVAL_ERR_STACK_UNDERFLOW;  break; }
			
			// Apply operator
			if(info.is_alpha) err = info.src.nary(args);  // Builtin Function
			else if(info.arity == 1) err = info.src.unary(args);  // Unary Operator
			else err = info.src.binary(args, valshf(args, 1));  // Binary Operator
			// Break on error
			if(err) break;
				
			// Deallocate used values
			for(int i = 1; i < info.arity; i++) valfree(valshf(args, i));
			stack.len++;  // Move stack back up one to include result
			
		}else{  // Constant or Variable load	
			void *val = INSTR_LOAD(exp, inst);  // Pointer to loaded value
			vec_inc_size(stack, nmsp_valctl.size);
			valclone(valshf(stack.ptr, stack.len++), val);  // Clone value onto top of stack
		}
	}
	
	// Should only be one remaining value after evaluation
	if(!err){
		if(stack.len == 0) err = EVAL_ERR_STACK_UNDERFLOW;
		else if(stack.len > 1) err = EVAL_ERR_STACK_SURPLUS;
	}
	
	if(err){  // On error, cleanup values on stack
		for(int i = 0; i < stack.len; i++) valfree(valshf(stack.ptr, i));
		return err;
	}
	
	// There will be one value left at the bottom of the stack
	return EXPR_ERR_OK;
}



// Add variable load to expression
static void expr_load_var(expr_t exp, var_t vr){
	// Check if variable already present
	int varid = -1;
	for(int i = 0; i < exp->vars.len; i++) if(exp->vars.ptr[i] == vr){
		varid = i;
		break;
	}
	
	if(varid < 0){
		varid = exp->vars.len;
		vecpush(exp->vars, vr);
	}
	vecpush(exp->instrs, INSTR_NEW_VAR(varid));
}

// Add constant load to expression
static void expr_load_const(expr_t exp, void *val){
	// Check if constant value is already present
	int constid = -1;
	for(size_t i = 0; i < exp->consts.len; i++){
		if(valequal(get_const(exp, i), val)){
			// If `val` is not placed into the constant list then it must be deallocated
			valfree(val);
			constid = i;
			break;
		}
	}
	
	if(constid < 0){
		constid = exp->consts.len;
		vecpush_sz(exp->consts, val, nmsp_valctl.size);
	}
	vecpush(exp->instrs, INSTR_NEW_CONST(constid));
}

/* Remove the last instruction if it is a const load
 * Additionally remove the corresponding const if no other loads use it
 *
 * Returns 1 if constant was removed and 0 otherwise
 * If the constant was removed the value is placed in `dest`
 * Otherwise the value of the constant is cloned into `dest`
 */
static bool expr_pop_const_load(expr_t exp, void *dest){
	// Get top instruction
	if(vecempty(exp->instrs)) return 0;
	instr_t instr = *veclast(exp->instrs);
	
	// Make sure top instruction is a constant load
	if(!INSTR_IS_CONST(instr)) return 0;
	vecpop(exp->instrs);  // Remove top instruction
	
	// Get associated constant
	size_t idx = INSTR_LOAD_INDEX(instr);
	void *val = get_const(exp, idx);
	
	// Check if any other instructions use this constant
	for(size_t i = 0; i < exp->instrs.len; i++){
		instr = exp->instrs.ptr[i];
		// If any other instruction uses this constant
		// We must clone the value into `dest`
		if(INSTR_IS_CONST(instr) && INSTR_LOAD_INDEX(instr) == idx){
			valclone(dest, val);
			return false;
		}
	}
	
	vecpop_sz(exp->consts, nmsp_valctl.size);  // If no other instructions use this constant then remove it
	valmove(dest, val);  // Place the value into `dest`
	return true;
}


// Check validity of parsed expression
static nmsp_err_t check_valid(expr_t exp){
	size_t height = 0;  // Track height of stack
	for(size_t i = 0; i < exp->instrs.len; i++){
		instr_t inst = exp->instrs.ptr[i];
		if(INSTR_IS_OPER(inst)){
			struct bltn_info_s info = nmsp_bltns[INSTR_OPERID(inst)];
			// Check that there are sufficient operators
			if(height < info.arity) return EVAL_ERR_STACK_UNDERFLOW;
			
			// Remove consumed arguments with result in first argument
			height -= info.arity - 1;
		}else height++;  // Load Instructions add one element
	}
	
	// No return value
	if(height == 0) return EVAL_ERR_STACK_UNDERFLOW;
	// More than one return value
	if(height > 1) return EVAL_ERR_STACK_SURPLUS;
	
	return EXPR_ERR_OK;
}






// Identify operator matching `str`
static bltn_t parse_infix_oper(const char *str, const char **endptr, bool is_unary){
	// Root of operator trees
	static struct oper_tree_s *binary_tree = NULL, *unary_tree = NULL;
	
	// Select tree to use
	struct oper_tree_s **root = is_unary ? &unary_tree : &binary_tree;
	size_t arity = 1 + !is_unary;
	// Construct operator tree if it doesn't exist for given type
	if(!*root){
		// Add each operator to tree
		struct bltn_info_s info;
		for(bltn_t opid = 0; (info = nmsp_bltns[opid]).name; opid++){
			// Only add operators (not builtin functions) of the specified type
			if(info.is_alpha || info.arity != arity) continue;
			
			// Iterate through name adding nodes into tree
			const char *name = info.name;
			const char *end = name + info.namelen;
			
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
	bltn_t opid = OPER_NULL;
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

// Apply single operator to value stack by appending
static nmsp_err_t apply_oper(expr_t exp, bltn_t opid){
	// If `nmsp_eval_on_parse` is set then try to
	// Evaluate constants using operator on the stack
	if(nmsp_eval_on_parse){
		struct bltn_info_s info = nmsp_bltns[opid];
		
		// Check for sufficient arguments
		if(exp->instrs.len < info.arity) return EVAL_ERR_STACK_UNDERFLOW;
		instr_t *inst = exp->instrs.ptr + exp->instrs.len - info.arity;
		
		// Check that all arguments are constant
		for(int i = 0; i < info.arity; i++) if(!INSTR_IS_CONST(inst[i])){
			vecpush(exp->instrs, INSTR_NEW_OPER(opid));  // Put operator onto instrs section
			return EXPR_ERR_OK;
		}
		
		// Pop constants off of instruction array and onto args
		valarr_def(args, info.arity);
		for(int i = info.arity - 1; i >= 0; i--) expr_pop_const_load(exp, valshf(args, i));
		
		// Apply operator
		nmsp_err_t err;
		if(info.is_alpha) err = info.src.nary(args);  // Builtin Functions
		else if(info.arity == 1) err = info.src.unary(args);  // Unary Operator
		else err = info.src.binary(args, valshf(args, 1));  // Binary Operator
		
		// Cleanup extra args
		for(int i = 1; i < info.arity; i++) valfree(valshf(args, i));
		
		if(err){
			valfree(args);  // On error, Deallocate first argument as well
			return err;
		}
		
		// On Success push result onto consts array and load it
		expr_load_const(exp, args);
		return EXPR_ERR_OK;
	}
	
	vecpush(exp->instrs, INSTR_NEW_OPER(opid));  // Put operator onto instrs section
	return EXPR_ERR_OK;
}


/* Place operator on stack from `opid`
 * If opid == -1 then place an open parenthesis
 * If opid == -2 then place a comma
 */
static void push_oper(oper_stack_t *opstk, int16_t opid){
	struct stk_oper_s elem;
	if(opid >= 0){
		struct bltn_info_s info = nmsp_bltns[opid];
		elem.is_block = 0;
		elem.prec_assoc = (info.prec << 1) | info.assoc | (info.arity == 1);
		elem.operid = (bltn_t)opid;
	}else{
		elem.is_block = 1;
		elem.is_comma = opid == -2;
	}
	vecpush(*opstk, elem);
}

// Place `op` onto `opstk` displacing operators as necessary and applying them to `exp`
static nmsp_err_t displace_opers(expr_t exp, oper_stack_t *opstk, int8_t prec){
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
	struct stk_oper_s *elem;
	for(;
		(elem = veclast(*opstk)) && !elem->is_block && (uint8_t)prec <= elem->prec_assoc;
		vecpop(*opstk)
	){	
		// Try to apply operator onto expression
		nmsp_err_t err;
		if(err = apply_oper(exp, elem->operid)) return err;
	}
	
	return EXPR_ERR_OK;
}

// Clears out parenthetical block from operator stack
// And calls function if necessary
static nmsp_err_t close_parenth(expr_t exp, oper_stack_t *opstk){
	// Remove all operators until close parenthesis
	nmsp_err_t err;
	if(err = displace_opers(exp, opstk, -1)) return err;
	
	// Consume commas to find number of values in parenthetical block
	size_t arity = 1;
	struct stk_oper_s *elem;
	while((elem = veclast(*opstk)) && elem->is_block && elem->is_comma){
		vecpop(*opstk);  arity++;
	}
	
	// Check for open parenthesis
	if(!(elem = veclast(*opstk)) || !elem->is_block){
		return PARSE_ERR_PARENTH_MISMATCH;  // No opening parenthesis so parenth mismatch
	}
	vecpop(*opstk);  // Remove open parenthesis
	
	// Check for function below open parenthesis
	struct bltn_info_s info;
	if((elem = veclast(*opstk))
	&& !elem->is_block
	&& (info = nmsp_bltns[elem->operid]).is_alpha
	){  // Treat parenthetical block as arguments to function
		vecpop(*opstk);  // Remove builtin function from `opstk`
		if(arity != info.arity) return PARSE_ERR_ARITY_MISMATCH;  // Check that arity matches
		apply_oper(exp, elem->operid);  // Apply operator onto values on stack
		
	}else{  // Treat parenthetical block as value
		if(arity > 1) return PARSE_ERR_BAD_COMMA;  // Arity must be 1
	}
	
	return EXPR_ERR_OK;
}

// Try to parse sequence of alphanumerics into builtin function or constant
static bltn_t parse_builtin(const char *str, const char **endptr){
	// Collect function name
	const char *name = str;
	while(isalnum(*str) || *str == '_') str++;
	size_t namelen = str - name;
	
	// Search Builtin Functions for match
	struct bltn_info_s info;
	for(bltn_t opid = 0; (info = nmsp_bltns[opid]).name; opid++) if(info.is_alpha){
		size_t minlen = namelen < info.namelen ? namelen : info.namelen;
		if(strncmp(info.name, name, minlen) == 0){
			if(endptr) *endptr = str;
			return opid;
		}
	}
	return OPER_NULL;
}

// Try to parse sequence of alphanumerics into variable
static var_t parse_var(const char *str, const char **endptr, namespace_t nmsp){
	if(endptr) *endptr = str;  // Set endptr to beginning
	
	// Collect variable name characters
	const char *name = str;
	while(isalnum(*str) || *str == '_') str++;
	size_t namelen = str - name;
	
	// Leave if nothing collected
	if(str == name) return NULL;
	
	var_t vr;
	if(nmsp && (
		// Query namespace for variable
		(vr = nmsp_get(nmsp, name, namelen)) ||
		// Otherwise create new variable with name
		(vr = nmsp_put(nmsp, name, namelen))
	)){
		if(endptr) *endptr = str;
		return vr;
	}else return NULL;
}

// Parses String as Expression
expr_t expr_parse(const char *str, const char **endptr, namespace_t nmsp, nmsp_err_t *err){
	// Initialize operator stack
	oper_stack_t opstk;
	vecinit(opstk, 8);
	
	// Initialize expression
	expr_t exp = expr_new(4, 4, 8);
	
	// Give err reference to prevent null dereferences
	nmsp_err_t tmperr;
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
			push_oper(&opstk, -1);  // Place open parenthesis on operator stack
			was_last_val = false;  // New expression so there is no last value
			continue;
		}else if(c == ','){
			if(parenth_depth == 0){  // Comma must be inside parentheses
				*err = PARSE_ERR_BAD_COMMA;
				break;
			}
			str++;  // Consume ','
			if(*err = displace_opers(exp, &opstk, -1)) break;  // Displace all operators until block
			
			push_oper(&opstk, -2);  // Place comma on operator stack
			was_last_val = false;  // New expression so there is no last value
			continue;
		}else if(c == ')'){
			str++;  // Consume ')'
			parenth_depth--;
			if(*err = close_parenth(exp, &opstk)) break;	
			was_last_val = true;
			continue;
		}
		
		// Try to parse operator
		bltn_t opid = parse_infix_oper(str, &after_tok, !was_last_val);
		if(opid != OPER_NULL && after_tok > str){
			// Get info for current operator
			struct bltn_info_s info = nmsp_bltns[opid];
			
			if(opstk.len > 0){
				// Get info for last operator
				struct bltn_info_s lst_info = nmsp_bltns[veclast(opstk)->operid];
				
				// Check that last operator isn't function
				if(lst_info.is_alpha){ *err = PARSE_ERR_FUNC_NOCALL;  break; }
				
				// When operator is unary
				// Check that previous operator is a block, unary, right-associative, or lower precedence
				if(info.arity == 1 && !(veclast(opstk)->is_block
				|| lst_info.arity == 1
				|| lst_info.assoc == OPER_RIGHT_ASSOC
				|| lst_info.prec < info.prec
				// Otherwise we have a unary operator coming after a binary operator of higher precedence
				)){ *err = PARSE_ERR_LOWPREC_UNARY;  break; }
			}
			
			
			// Only displace operators if binary operator
			if(info.arity == 2){  // If operator follows val it is binary
				if(*err = displace_opers(exp, &opstk, (int8_t)(info.prec))) break;
			}
			push_oper(&opstk, opid);  // Place operator at top of stack
			str = after_tok;  // Move string forward
			was_last_val = false;  // Operator is not a value
			continue;
		}
		
		
		// Try to parse constant
		valdef(val);
		if(nmsp_valctl.parse(val, str, &after_tok)){
			// Cannot have two values in a row
			if(was_last_val){ *err = PARSE_ERR_MISSING_OPERS;  break; }
			
			expr_load_const(exp, val);  // Load constant into expression
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Try to parse builtin function or constant name
		opid = parse_builtin(str, &after_tok);
		if(opid != OPER_NULL && after_tok > str){
			// Cannot have two values in a row
			if(was_last_val){ *err = PARSE_ERR_MISSING_OPERS;  break; }
			
			if(nmsp_bltns[opid].arity == 0){  // Place constant on value stack (i.e. expression)
				valclone(val, nmsp_bltns[opid].src.value);
				expr_load_const(exp, val);
			}else{  // Place function on operator stack
				push_oper(&opstk, opid);
			}
			was_last_val = true;
			str = after_tok;
			continue;
		}
		
		// Try to parse variable name
		// Collect alphanumerics and _
		var_t vr;
		if(vr = parse_var(str, &after_tok, nmsp)){
			// Cannot have two values in a row
			if(was_last_val){ *err = PARSE_ERR_MISSING_OPERS;  break; }
			
			expr_load_var(exp, vr);  // Load variable into expression
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		break;  // Failed to parse token
	}
	
	// Move endpointer to after parsed section
	if(endptr) *endptr = str;
	
	// On error cleanup and leave
	if(*err){
		// Deallocate operator stack and exp
		expr_free(exp);
		vecfree(opstk);
		return NULL;
	}
	
	// Clear out remaining operators on the operator stack
	struct stk_oper_s op;
	while(!vecempty(opstk)){
		op = vecpop(opstk);
		// Make sure no open parenths are left
		if(op.is_block){ *err = PARSE_ERR_PARENTH_MISMATCH;  break; }
		// Or function calls
		if(nmsp_bltns[op.operid].is_alpha){ *err = PARSE_ERR_FUNC_NOCALL;  break; }
		
		// Apply operator to expression
		if(*err = apply_oper(exp, op.operid)) break;
	}
	vecfree(opstk);  // Deallocate stack
	
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

