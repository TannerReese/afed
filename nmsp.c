#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>

// Utilities
#include "util/vec.h"  // Vector operations
#include "util/queue.h"  // Queue of variables
#include "util/ptree.h"  // Tree of operators used to find longest prefix match

#include "nmsp.h"



/* Instruction
 * -------------------
 * The instr_t type holds a single 16-bit instruction
 * which will be evaluated by the virtual machine (expr_eval).
 * The machine maintains a stack and applies operations to it.
 * 
 * Every instruction is either a load or apply instruction.
 * The load instructions cause a value to be pushed onto the stack.
 * The apply instructions cause values to be popped
 * and some arithmetic to be applied to them.
 * 
 * Apply Instructions:
 *   OPER : Represents one of the operations or functions in the `nmsp_bltns` array.
 *   CALL : A user-defined function is called on some number of values on the stack.
 *          The function is identified using its index in `vars`
 * Load Instructions:
 *   CONST : Loads a constant value stored in `consts` onto the stack
 *   VAR : Loads the cached value of a variable from `vars` onto the stack
 *   ARG : Loads the value of an argument from `args` onto the stack
 */
typedef uint16_t instr_t;


// Create apply instruction using bltn_t or variable index
#define INSTR_NEW_OPER(op) (0x8000 | ((op) & 0x3fff))
#define INSTR_NEW_CALL(idx) (0xc000 | ((idx) & 0x3fff))

#define INSTR_IS_APPLY(inst) ((inst) & 0x8000)
#define INSTR_IS_BLTN(inst) (((inst) & 0xc000) == 0x8000)
#define INSTR_IS_CALL(inst) (((inst) & 0xc000) == 0xc000)

#define INSTR_BLTNID(inst) ((inst) & 0x3fff)
#define INSTR_CALL_VAR(exp, inst) ((exp)->vars.ptr[(inst) & 0x3fff])  // Get variable pointer for CALL
// Get number of values consumed by apply instruction
#define INSTR_ARITY(exp, inst) (INSTR_IS_BLTN(inst) ?\
	nmsp_bltns[INSTR_BLTNID(inst)].arity :\
	INSTR_CALL_VAR(exp, inst)->arity\
)

// Create instructions to load a variable, constant, or argument
#define INSTR_NEW_CONST(idx) ((idx) & 0x1fff)
#define INSTR_NEW_VAR(idx) (0x4000 | ((idx) & 0x1fff))
#define INSTR_NEW_ARG(idx) (0x2000 | (idx) & 0x1fff)

#define INSTR_IS_LOAD(inst) (!((inst) & 0x8000))
#define INSTR_IS_CONST(inst) (!((inst) & 0xe000))
#define INSTR_IS_VAR(inst) (((inst) & 0xe000) == 0x4000)
#define INSTR_IS_ARG(inst) (((inst) & 0xe000) == 0x2000)

// Get index that this instruction loads from
#define INSTR_LOAD_INDEX(inst) ((inst) & 0x1fff)
// Get constant, variable, or argument from expression
#define INSTR_LOAD(exp, inst) (INSTR_IS_VAR(inst) ?\
	(exp)->vars.ptr[INSTR_LOAD_INDEX(inst)]->cached :\
	valshf((INSTR_IS_CONST(inst) ? (exp)->consts.ptr : (exp)->args), INSTR_LOAD_INDEX(inst))\
)



// Variable / Namespace methods
// -----------------
// Forward declaration of expression type
struct expr_s;
typedef struct expr_s *expr_t;

typedef uint32_t hash_t;

struct var_s {
	expr_t expr;  // Expression that defines this variables
	size_t arity;  // Number of arguments to expression (zero if constant)
	
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
	
	// Pointer to memory containing arguments
	void *args;
	
	// Instructions to Run
	vec_t(instr_t) instrs;
};

// Perform pointer arithmetic with value pointer
#define valshf(ptr, shf) ((ptr) + (shf) * nmsp_valctl.size)
// Access constant value at index i
#define get_const(exp, i) valshf((exp)->consts.ptr, i)
#define set_const(exp, i, val) valmove(get_const(exp, i), val)
#define get_arg(exp, i) valshf((exp)->args, i)


// Create expression with the given capacities for each section
static expr_t expr_new(size_t varcap, size_t constcap, size_t instrcap);
// Deallocates memory allocated to expression and any constants it holds
static void expr_free(expr_t exp);

typedef vec_t(void) valstk_t;  // Stack of values used during evaluation
/* Evaluate expression using stack-based virtual machine
 * Each instruction of the `exp->instrs` vector will
 * load or manipulate the values in `stack`
 */
static nmsp_err_t expr_eval(expr_t exp, valstk_t *stack);



/* Put variable into expr variable section if not already present
 * Return the index of the variable
 */
static int expr_put_var(expr_t exp, var_t vr);
// Put variable into expression and add load instruction for it
#define expr_load_var(exp, vr) vecpush((exp)->instrs, INSTR_NEW_VAR(expr_put_var(exp, vr)))
// Put variable into expression and add call instruction for it
#define expr_call_var(exp, vr) vecpush((exp)->instrs, INSTR_NEW_CALL(expr_put_var(exp, vr)))

/* Place variable in expr variable section if not already present
 * Return the index of the constant
 */
static int expr_put_const(expr_t exp, void *val);
// Put constant into expression and add load instruction for it
#define expr_load_const(exp, val) vecpush((exp)->instrs, INSTR_NEW_CONST(expr_put_const(exp, val)))

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
	bool is_block : 1;
	// If this element is a comma then it will be displaced only by close parenthesis
	bool is_comma : 1;
	// If this element is a user-defined function call defined by `vr`
	bool is_var : 1;
	
	// Stores Precedence in higher 7 bits and
	// Associativity in the least significant bit
	uint8_t prec_assoc;
	
	union {
		bltn_t bltnid;  // Id of operator
		var_t var;
	} src;
};

typedef vec_t(struct stk_oper_s) oper_stack_t;  // Operator stack


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
static bltn_t parse_oper(const char *str, const char **endptr, bool is_unary);

/* Take an operator and apply it to the values in `exp`
 * Usually, this means the operator instruction is added to `exp->instrs`
 * 
 * If evaluation-on-parsing is turned on and the loaded values are constants
 * Then the operator will be immediately evaluated
 * And the result loaded onto `exp->instrs` as a constant
 */
bool nmsp_eval_on_parse = true;  // Whether to evaluate constants while parsing
static nmsp_err_t apply_oper(expr_t exp, struct stk_oper_s op);

/* Place operator with id `opid` on stack
 * If opid == -1 then place an open parenthesis
 * If opid == -2 then place a comma
 */
static void push_oper(oper_stack_t *opstk, int16_t opid);

/* Place user-defined function call defined by `vr`
 * onto the operator stack
 */
static void push_call(oper_stack_t *opstk, var_t vr);

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


typedef vec_t(struct {
	size_t len;  // Length of argument
	const char *arg;
}) arg_list_t;

/* Read sequence of alphanumerics and '_' as a name
 * Return index of matching argument from `args` or -1 if none found
 */
static int parse_arg(const char *name, size_t namelen, arg_list_t args);

/* Find builtin constant or function matching `name`
 * If none are found OPER_NULL is returned
 */
static bltn_t parse_builtin(const char *name, size_t namelen);

/* Find and return variable which matches `name`
 * If no such variable exists then create one
 */
static var_t parse_var(const char *name, size_t namelen, namespace_t nmsp);

/* Primary method for parsing expression
 * Parses as much as possuble of the string
 * If `err` is not NULL then any errors are stored in it
 */
static expr_t expr_parse(const char *str, const char **endptr, arg_list_t args, namespace_t nmsp, nmsp_err_t *err);





// Returns a string containing a description of errors
const char *nmsp_strerror(nmsp_err_t err){
	switch(err){
		case NMSP_ERR_OK: return "NMSP_ERR_OK: Successful";
		
		// Evaluation Errors
		case EVAL_ERR_STACK_UNDERFLOW: return "EVAL_ERR_STACK_UNDERFLOW: Values popped when Stack empty";
		case EVAL_ERR_STACK_SURPLUS: return "EVAL_ERR_STACK_SURPLUS: Too Many values on stack at end of program";
		case EVAL_ERR_NO_EXPR: return "EVAL_ERR_NO_EXPR: Referenced Variable didn't have expression";
		case EVAL_ERR_VAR_NOT_FUNC: return "EVAL_ERR_VAR_NOT_FUNC: A Variable cannot be called";
		case EVAL_ERR_NO_ARGS: return "EVAL_ERR_NO_ARGS: Cannot load arguments as there is no argument list";
		
		// Parsing Errors
		case PARSE_ERR_PARENTH_MISMATCH: return "PARSE_ERR_PARENTH_MISMATCH: Missing open or close parenthesis";
		case PARSE_ERR_LOWPREC_UNARY: return "PARSE_ERR_LOWPREC_UNARY: Unary operator follows Binary of Higher Precedence";
		case PARSE_ERR_ARITY_MISMATCH: return "PARSE_ERR_ARITY_MISMATCH: Wrong number of arguments given to function";
		case PARSE_ERR_BAD_COMMA: return "PARSE_ERR_BAD_COMMA: Comma in wrong location";
		case PARSE_ERR_FUNC_NOCALL: return "PARSE_ERR_FUNC_NOCALL: Function present but not called";
		
		// Produce after parsing produces invalid expression
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
		vr->expr->args = NULL;
		vr->err = expr_eval(vr->expr, &stack);
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
	vr->arity = 0;
	vr->is_cached = false;
	vr->cached = NULL;
	vr->err = NMSP_ERR_OK;
	
	// Store name of variable
	vr->name = key;
	vr->namelen = keylen;
	vr->hash = hash(key, keylen);
	
	// Will be used during nmsp_define by find_circ
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



// Methods used by nmsp_define
// ----------------------------

// Find circular dependency
static bool find_circ(namespace_t nmsp, expr_t start, var_t target){
	if(!start || !target) return false;
	
	// Clear out any previous dependency tree
	nmsp->circ_root = NULL;
	for(var_t v = nmsp->head; v; v = v->next) v->used_by = NULL;
	
	// Initialize with exp's immediate dependencies
	struct queue_s q = queue_new(start->vars.len << 1);
	queue_push(&q, (void**)start->vars.ptr, start->vars.len);
	// Set their reference to `target`
	for(int i = 0; i < start->vars.len; i++) start->vars.ptr[i]->used_by = target;
	
	// Iterate over variables checking their dependencies
	while(q.len > 0){  // While there are remaining variables to check
		// Get variable
		var_t vr = queue_pop(&q);
		
		// Check if it matches the root variable
		if(target == vr){
			nmsp->circ_root = target;
			queue_free(q);  // Cleanup queue
			return true;  // Circular dependency found
		}
		
		// If variable's expression has no variables go to next
		if(!vr->expr || vr->expr->vars.len == 0) continue;
		
		var_t *deps = vr->expr->vars.ptr;
		size_t deplen = vr->expr->vars.len;
		// Add all variables used by `vr` to the queue
		queue_push(&q, (void**)deps, deplen);
		// If `used_by` is not already set
		// Set the `used_by` pointer to point to the parent node in the dependency tree
		for(int i = 0; i < deplen; i++) if(!deps[i]->used_by) deps[i]->used_by = vr;
	}
	
	queue_free(q);  // Free queue
	return false;  // No circular dependency
}

// Parse the name of the variable before an expression
// If a function the argument list will also be found
static size_t parse_label(const char *str, const char **endptr, arg_list_t *args){
	if(endptr) *endptr = str;
	
	// Collect label characters
	const char *lbl = str;
	while(isalnum(*str) || *str == '_') str++;
	size_t lbl_len = str - lbl;
	if(lbl_len == 0) return 0;
	
	while(isblank(*str)) str++;  // Skip blankspace after label
	
	// Check for argument list
	if(*str == '('){
		// Parse arguments
		do{
			str++;
			while(isspace(*str)) str++;  // Skip whitespace before argument
			const char *start = str;
			while(isalnum(*str) || *str == '_') str++;
			size_t len = str - start;
			while(isspace(*str)) str++;  // Skip whitespace after argument
			
			if(len > 0){
				vecinc(*args);
				args->ptr[args->len].arg = start;
				args->ptr[args->len].len = len;
				args->len++;
			}else return 0;
		}while(*str == ',');
		if(*str != ')') return 0;  // Check for parenthesis at the end
		str++;  // Consume ')'
		while(isblank(*str)) str++;  // Skip blankspace after args
	}
	
	// Check for colon after label
	if(*str != ':') return 0;
	str++;  // Consume ':'
	
	if(endptr) *endptr = str;
	return lbl_len;
}

var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, nmsp_err_t *err){
	// Parse Label
	// ------------
	// Collect label characters
	while(isblank(*str)) str++;  // Skip whitespace before
	const char *lbl = str;
	
	// Try to parse label
	arg_list_t args;
	vecinit(args, 0);
	size_t lbl_len = parse_label(str, &str, &args);
	
	if(lbl_len == 0){  // No Label was parsable
		str = lbl;
		vecfree(args);
		lbl = NULL;
	}
	size_t arity = args.len;
	
	// Parse Expression
	// -----------------
	expr_t exp = expr_parse(str, endptr, args, nmsp, err);
	vecfree(args);  // Free argument vector
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
		
		if(oldvar->arity != arity){  // Check for matching arity
			*err = PARSE_ERR_ARITY_MISMATCH;
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
static nmsp_err_t expr_eval(expr_t exp, valstk_t *stack){
	size_t start_len = stack->len;  // Keep track of how many values on stack to begin
	// If no expression is provided
	if(!exp) return EVAL_ERR_NO_EXPR;
	
	// Evaluate any variable dependencies (that are not functions)
	for(size_t i = 0; i < exp->vars.len; i++){
		var_t vr = exp->vars.ptr[i];
		// Can't preemptively evaluate functions
		if(vr->arity > 0) continue;
		
		// Allocate space for variable if needed
		if(!vr->cached) vr->cached = malloc(nmsp_valctl.size);
		// Evaluate expression if no cached variable
		if(!vr->is_cached){
			vr->err = expr_eval(vr->expr, stack);
			// Place value into cache if no error occurred
			if(!vr->err) valmove(vr->cached, vecpop_sz(*stack, nmsp_valctl.size));
		}
		
		if(vr->err) return vr->err;
	}
	
	nmsp_err_t err = NMSP_ERR_OK;  // Track any errors that happen
	
	// Run Instructions
	for(size_t i = 0; i < exp->instrs.len; i++){
		instr_t inst = exp->instrs.ptr[i];
		
		if(INSTR_IS_BLTN(inst)){  // Operation or User-Define Function call
			struct bltn_info_s info = nmsp_bltns[INSTR_BLTNID(inst)];
			void *args = vecremove_sz(*stack, info.arity, nmsp_valctl.size);
			// Check that there are enough arguments
			if(!args){ err = EVAL_ERR_STACK_UNDERFLOW;  break; }
			
			// Apply operator
			if(info.is_word) err = info.src.nary(args);  // Builtin Function
			else if(info.arity == 1) err = info.src.unary(args);  // Unary Operator
			else err = info.src.binary(args, valshf(args, 1));  // Binary Operator
			// Break on error
			if(err) break;
				
			// Deallocate used values
			for(int i = 1; i < info.arity; i++) valfree(valshf(args, i));
			stack->len++;  // Move stack back up one to include result
			
		}else if(INSTR_IS_CALL(inst)){  // User-Defined Function call
			var_t func = INSTR_CALL_VAR(exp, inst);
			// Check that `func` is a function
			if(func->arity == 0){ err = EVAL_ERR_VAR_NOT_FUNC;  break; }
			// Check that there are enough arguments
			if(stack->len < func->arity){ err = EVAL_ERR_STACK_UNDERFLOW;  break; }
			
			// Check that function's expression is defined
			if(!func->expr){ err = EVAL_ERR_NO_EXPR;  break; }
			
			// Give arguments pointer to expression and evaluate it using current stack
			func->expr->args = valshf(stack->ptr, stack->len - func->arity);
			if(err = expr_eval(func->expr, stack)) break;
			
			// Move result to first argument and reduce stack
			valmove(func->expr->args, veclast_sz(*stack, nmsp_valctl.size));
			vecremove_sz(*stack, func->arity, nmsp_valctl.size);
			
		}else{  // Constant, Variable, or Argument load	
			if(INSTR_IS_ARG(inst) && !exp->args){ err = EVAL_ERR_NO_ARGS;  break; }
			void *val = INSTR_LOAD(exp, inst);  // Pointer to loaded value
			vecinc_sz(*stack, nmsp_valctl.size);
			valclone(valshf(stack->ptr, stack->len++), val);  // Clone value onto top of stack
		}
	}
	
	// Should only be one remaining value after evaluation
	if(!err){
		if(stack->len == start_len) err = EVAL_ERR_STACK_UNDERFLOW;
		else if(stack->len > start_len + 1) err = EVAL_ERR_STACK_SURPLUS;
	}
	
	if(err){  // On error, cleanup values on stack
		for(int i = 0; i < stack->len; i++) valfree(valshf(stack->ptr, i));
		return err;
	}
	
	// There will be one value left at the bottom of the stack
	return NMSP_ERR_OK;
}



// Add variable load to expression
static int expr_put_var(expr_t exp, var_t vr){
	// Check if variable already present
	for(int i = 0; i < exp->vars.len; i++) if(exp->vars.ptr[i] == vr) return i;
	
	vecpush(exp->vars, vr);
	return exp->vars.len - 1;
}

// Add constant load to expression
static int expr_put_const(expr_t exp, void *val){
	// Check if constant value is already present
	for(size_t i = 0; i < exp->consts.len; i++){
		if(valequal(get_const(exp, i), val)){
			// If `val` is not placed into the constant list then it must be deallocated
			valfree(val);
			return i;
		}
	}
	
	vecpush_sz(exp->consts, val, nmsp_valctl.size);
	return exp->consts.len - 1;
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
		if(INSTR_IS_APPLY(inst)){
			size_t arity = INSTR_ARITY(exp, inst);
			// Check that there are sufficient operators
			if(height < arity) return PARSE_ERR_MISSING_VALUES;
			
			// Remove consumed arguments with result in first argument
			height -= arity - 1;
		}else if(INSTR_IS_LOAD(inst)) height++;  // Load Instructions add one element
	}
	
	// No return value
	if(height == 0) return PARSE_ERR_MISSING_VALUES;
	// More than one return value
	if(height > 1) return PARSE_ERR_MISSING_OPERS;
	
	return NMSP_ERR_OK;
}






// Identify operator matching `str`
static bltn_t parse_oper(const char *str, const char **endptr, bool is_unary){
	// Root of operator trees
	static ptree_t binary_tree = ptree_new(), unary_tree = ptree_new();
	
	// Select tree to use
	ptree_t *root = is_unary ? &unary_tree : &binary_tree;
	size_t arity = 1 + !is_unary;
	// Construct operator tree if it doesn't exist for given type
	if(!*root){
		// Add each operator to tree
		struct bltn_info_s info;
		for(bltn_t id = 0; (info = nmsp_bltns[id]).name; id++){
			if(!info.is_word && info.arity == arity)
				ptree_putn(root, info.name, info.namelen, id);
		}
	}
	
	// Set endptr to beginning for empty case
	if(endptr) *endptr = str;
	
	// Use operator tree to identify string
	return (bltn_t)ptree_get(*root, str, endptr);
}

// Apply single operator to value stack by appending
static nmsp_err_t apply_oper(expr_t exp, struct stk_oper_s op){
	// If `nmsp_eval_on_parse` is set then try to
	// Evaluate constants using operator on the stack
	if(nmsp_eval_on_parse && !op.is_var){
		struct bltn_info_s info = nmsp_bltns[op.src.bltnid];
		
		// Check for sufficient arguments
		if(exp->instrs.len < info.arity) return EVAL_ERR_STACK_UNDERFLOW;
		instr_t *inst = exp->instrs.ptr + exp->instrs.len - info.arity;
		
		// Check that all arguments are constant
		for(int i = 0; i < info.arity; i++) if(!INSTR_IS_CONST(inst[i])){
			vecpush(exp->instrs, INSTR_NEW_OPER(op.src.bltnid));  // Put operator onto instrs section
			return NMSP_ERR_OK;
		}
		
		// Pop constants off of instruction array and onto args
		valarr_def(args, info.arity);
		for(int i = info.arity - 1; i >= 0; i--) expr_pop_const_load(exp, valshf(args, i));
		
		// Apply operator
		nmsp_err_t err;
		if(info.is_word) err = info.src.nary(args);  // Builtin Functions
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
		return NMSP_ERR_OK;
	}
	
	if(op.is_var){  // Call variable when user-defined function
		expr_call_var(exp, op.src.var);
	}else{  // Apply operator when builtin
		vecpush(exp->instrs, INSTR_NEW_OPER(op.src.bltnid));
	}
	return NMSP_ERR_OK;
}


/* Place operator on stack from `opid`
 * If opid == -1 then place an open parenthesis
 * If opid == -2 then place a comma
 */
static void push_oper(oper_stack_t *opstk, int16_t opid){
	struct stk_oper_s elem;
	elem.is_var = false;
	if(opid >= 0){
		struct bltn_info_s info = nmsp_bltns[opid];
		elem.is_block = false;
		elem.prec_assoc = (info.prec << 1) | info.assoc | (info.arity == 1);
		elem.src.bltnid = (bltn_t)opid;
	}else{
		elem.is_block = true;
		elem.is_comma = opid == -2;
	}
	vecpush(*opstk, elem);
}

// Place a user-defined function call on the stack
static void push_call(oper_stack_t *opstk, var_t vr){
	struct stk_oper_s elem;
	elem.is_var = true;
	elem.is_block = false;
	elem.is_comma = false;
	elem.src.var = vr;
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
		if(err = apply_oper(exp, *elem)) return err;
	}
	
	return NMSP_ERR_OK;
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
	&& (
	    elem->is_var // User-Defined Function
	||  (!elem->is_block && (info = nmsp_bltns[elem->src.bltnid]).is_word)  // Builtin Function
	)){  // Treat parenthetical block as arguments to function
		vecpop(*opstk);  // Remove function from `opstk`
		if(elem->is_var){
			var_t vr = elem->src.var;
			// If variable hasn't been initialized set arity
			if(!vr->expr) vr->arity = arity;
			// Otherwise check that arity matches
			else if(arity != vr->arity) return PARSE_ERR_ARITY_MISMATCH;
		}else{
			if(arity != info.arity) return PARSE_ERR_ARITY_MISMATCH;  // Check that arity matches
		}
		apply_oper(exp, *elem);  // Apply operator onto values on stack
		
	}else{  // Treat parenthetical block as value
		if(arity > 1) return PARSE_ERR_BAD_COMMA;  // Arity must be 1
	}
	
	return NMSP_ERR_OK;
}

// Try to parse sequence of alphanumerics into argument name
static int parse_arg(const char *name, size_t namelen, arg_list_t args){
	for(int i = 0; i < args.len; i++){
		if(namelen == args.ptr[i].len && strncmp(name, args.ptr[i].arg, namelen) == 0) return i;
	}
	return -1;
}

// Try to parse sequence of alphanumerics into builtin function or constant
static bltn_t parse_builtin(const char *name, size_t namelen){
	// Search Builtin Functions for match
	struct bltn_info_s info;
	for(bltn_t id = 0; (info = nmsp_bltns[id]).name; id++) if(info.is_word){
		if(namelen == info.namelen
		&& strncmp(info.name, name, namelen) == 0
		) return id;
	}
	return OPER_NULL;
}

// Try to parse sequence of alphanumerics into variable
static var_t parse_var(const char *name, size_t namelen, namespace_t nmsp){
	var_t vr;
	if(nmsp && (
		// Query namespace for variable
		(vr = nmsp_get(nmsp, name, namelen)) ||
		// Otherwise create new variable with name
		(vr = nmsp_put(nmsp, name, namelen))
	)){
		return vr;
	}else return NULL;
}

// Parses String as Expression
expr_t expr_parse(const char *str, const char **endptr, arg_list_t args, namespace_t nmsp, nmsp_err_t *err){
	// Initialize operator stack
	oper_stack_t opstk;
	vecinit(opstk, 8);
	
	// Initialize expression
	expr_t exp = expr_new(4, 4, 8);
	
	// Give err reference to prevent null dereferences
	nmsp_err_t tmperr;
	if(!err) err = &tmperr;
	*err = NMSP_ERR_OK;
	
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
		bltn_t opid = parse_oper(str, &after_tok, !was_last_val);
		if(opid != OPER_NULL && after_tok > str){
			// Get info for current operator
			struct bltn_info_s info = nmsp_bltns[opid];
			
			struct stk_oper_s *last = veclast(opstk);
			if(last && !last->is_block){  // For non-block previous operator
				struct bltn_info_s lst_info;
				
				// Check that last operator isn't function
				if(last->is_var  // User-defined function
				|| (lst_info = nmsp_bltns[last->src.bltnid]).is_word  // Builtin function
				){ *err = PARSE_ERR_FUNC_NOCALL;  break; }
				
				// When operator is unary
				// Check that previous operator is a unary, right-associative, or lower precedence
				if(info.arity == 1 && !(lst_info.arity == 1
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
		
		
		// Collect word characters
		after_tok = str;
		while(isalnum(*after_tok) || *after_tok == '_') after_tok++;
		size_t namelen = after_tok - str;
		if(namelen == 0) break;  // Unknown token
		
		// Try to parse argument name
		int argind;
		if((argind = parse_arg(str, namelen, args)) >= 0){
			// Cannot have two values in a row
			if(was_last_val){ *err = PARSE_ERR_MISSING_OPERS;  break; }
			
			vecpush(exp->instrs, INSTR_NEW_ARG(argind));
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Try to parse builtin function or constant name
		opid = parse_builtin(str, namelen);
		if(opid != OPER_NULL){
			// Cannot have two values in a row
			if(was_last_val){ *err = PARSE_ERR_MISSING_OPERS;  break; }
			
			if(nmsp_bltns[opid].arity == 0){  // Place constant on value stack (i.e. expression)
				valclone(val, nmsp_bltns[opid].src.value);
				expr_load_const(exp, val);
			}else{  // Place function on operator stack
				push_oper(&opstk, opid);
			}
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Try to parse variable name
		// Collect alphanumerics and _
		var_t vr;
		if(vr = parse_var(str, namelen, nmsp)){
			// Cannot have two values in a row
			if(was_last_val){ *err = PARSE_ERR_MISSING_OPERS;  break; }
			
			// Check if next character is '('
			const char *tmp = after_tok;
			while(parenth_depth > 0 ? isspace(*tmp) : isblank(*tmp)) tmp++;
			if(*tmp == '('){  // Treat `vr` as a user-defined function
				// If already defined check that `vr` is a function
				if(vr->expr && vr->arity == 0){ *err = PARSE_ERR_MISSING_OPERS;  break; }
				push_call(&opstk, vr);
			}else{  // Treat `vr` as a variable
				expr_load_var(exp, vr);  // Load variable into expression
			}
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
		if(nmsp_bltns[op.src.bltnid].is_word){ *err = PARSE_ERR_FUNC_NOCALL;  break; }
		
		// Apply operator to expression
		if(*err = apply_oper(exp, op)) break;
	}
	vecfree(opstk);  // Deallocate stack
	
	if(*err){
		// Deallocate expression on error
		expr_free(exp);
		return NULL;
	}
	
	// Check that expression will evaluate appropriately
	if(*err = check_valid(exp)){
		expr_free(exp);
		return NULL;
	}
	
	return exp;
}

