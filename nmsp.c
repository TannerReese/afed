#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>

// Utilities
#include "util/vec.h"  // Vector operations
#include "util/queue.h"  // Queue of variables

#include "nmsp.h"
#include "bltn.h"
#include "mcode.h"



typedef uint32_t hash_t;

struct var_s {
	mcode_t code;  // Code Block defining this variable
	bool has_impl : 1;  // Whether code block has been filled
	
	// Array of dependencies of `code`
	size_t deplen;
	var_t *deps;
	
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
/* Put given `code` into namespace under name `key`
 * Without performing checks for redefinition or circular dependency
 */
static var_t place_var_unsafe(namespace_t nmsp, const char *key, size_t keylen, mcode_t code, bool isimpl);
/* Try to set `code` after variable has been already been added to namespace
 * Return true if unable to set `code`
 */
static bool var_calc_deps(namespace_t nmsp, var_t vr);

// Queue methods (used for dependency checking)
// Return true if `start` depends on variable `target`
static bool find_circ(namespace_t nmsp, var_t start);




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
	// If this element is a user-defined function call defined by `code`
	bool is_code : 1;
	// If ths element is an alphanumeric 
	bool is_oper : 1;
	
	// Stores Precedence in higher 7 bits and
	// Associativity in the least significant bit
	uint8_t prec_assoc;
	
	union {
		bltn_t bltn;  // Alphanumerically Named Builtin Function
		bltn_oper_t oper;  // Builtin Operator
		mcode_t code; // User-defined function
	};
};

typedef vec_t(struct stk_oper_s) oper_stack_t;  // Operator stack


/* Take an operator and apply it to the values in `exp`
 * Usually, this means the operator instruction is added to `exp->instrs`
 * 
 * If evaluation-on-parsing is turned on and the loaded values are constants
 * Then the operator will be immediately evaluated
 * And the result loaded onto `exp->instrs` as a constant
 */
bool nmsp_eval_on_parse = true;  // Whether to evaluate constants while parsing

// Place builtin operator onto stack
static void push_oper(oper_stack_t *opstk, bltn_oper_t oper);

/* Place user-defined function call defined
 * by code block `code` onto the operator stack
 */
static void push_call(oper_stack_t *opstk, mcode_t code);

// Apply given stk_oper_s onto the code
static nmsp_err_t apply_oper(mcode_t code, struct stk_oper_s elem);

/* Pops operators from the operator stack (i.e. `opstk`)
 * while they have lower precedence than `prec`
 * Each popped operator is applied to the value stack
 * (i.e. `exp->instrs`) using `apply_oper`
 * Search "Shunting Yard Algorithm" for explanation
 */
static nmsp_err_t displace_opers(mcode_t code, oper_stack_t *opstk, int8_t prec);

/* Called when a ')' is encountered
 * Removes any remaining operators
 * And checks for function calls
 */
static nmsp_err_t close_parenth(mcode_t code, oper_stack_t *opstk);

/* Read sequence of alphanumerics and '_' as a name
 * Return index of matching argument from `args` or -1 if none found
 */
static int parse_arg(const char *name, size_t namelen, const char *args);

/* Find and return variable which matches `name`
 * If no such variable exists then create one
 */
static var_t parse_var(const char *name, size_t namelen, namespace_t nmsp);

/* Primary method for parsing expression
 * Parses as much as possuble of the string
 * If `err` is not NULL then any errors are stored in it
 */
static nmsp_err_t mcode_parse(mcode_t code, const char *str, const char **endptr, const char *args, namespace_t nmsp);





// Returns a string containing a description of errors
const char *nmsp_strerror(nmsp_err_t err){
	switch(err){
		case NMSP_ERR_OK: return "NMSP_ERR_OK: Successful";
		
		// Parsing Errors
		case NMSP_ERR_PARENTH_MISMATCH: return "NMSP_ERR_PARENTH_MISMATCH: Missing open or close parenthesis";
		case NMSP_ERR_LOWPREC_UNARY: return "NMSP_ERR_LOWPREC_UNARY: Unary operator follows Binary of Higher Precedence";
		case NMSP_ERR_ARITY_MISMATCH: return "NMSP_ERR_ARITY_MISMATCH: Wrong number of arguments given to function";
		case NMSP_ERR_BAD_COMMA: return "NMSP_ERR_BAD_COMMA: Comma in wrong location";
		case NMSP_ERR_FUNC_NOCALL: return "NMSP_ERR_FUNC_NOCALL: Function present but not called";
		
		// Produce after parsing produces invalid expression
		case NMSP_ERR_MISSING_VALUES: return "NMSP_ERR_MISSING_VALUES: Operator is missing argument";
		case NMSP_ERR_MISSING_OPERS: return "NMSP_ERR_MISSING_OPERS: Multiple values without operator between";
		case NMSP_ERR_EXTRA_CONT: return "NMSP_ERR_EXTRA_CONT: Values present after expression";
		
		// Insertion Errors
		case INSERT_ERR_REDEF: return "INSERT_ERR_REDEF: Variable already exists";
		case INSERT_ERR_CIRC: return "INSERT_ERR_CIRC: Variable depends on itself";
	}
	
	return "NMSP_ERR: Unknown Error";
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
arith_t nmsp_var_value(var_t vr, arith_err_t *errp){
	if(vr->has_impl) return mcode_eval(vr->code, NULL, errp);
	if(errp) *errp = MCODE_ERR_INCOMPLETE_CODE;
	return NULL;
}

// Print variable value to a file
int nmsp_var_fprint(FILE *stream, var_t vr){
	// Calculate value
	arith_err_t err = MCODE_ERR_OK;
	void *val = mcode_eval(vr->code, NULL, &err);
	
	// Print value
	if(err) return fprintf(stream, "ERR %i", err);
	else return arith_print(stream, val);
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
		
		// Free any code block the variable might have
		if(vr->code) mcode_free(vr->code);
		
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
static var_t place_var_unsafe(namespace_t nmsp, const char *key, size_t keylen, mcode_t code, bool isimpl){
	var_t vr = malloc(sizeof(struct var_s));
	// Set Expression with no cached value yet
	vr->code = code;
	vr->has_impl = isimpl;
	
	// Calculate dependencies of code block
	vr->deplen = 0;  vr->deps = NULL;  // Initialize empty dependency list
	var_calc_deps(nmsp, vr);
	
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

static bool var_calc_deps(namespace_t nmsp, var_t vr){
	// If variable already has no implementation
	// Then dependencies can't be calculated
	if(!vr->has_impl) return true;
	
	// Get dependencies
	size_t deplen;
	mcode_t *code_deps = mcode_deplist(vr->code, &deplen);
	
	// Try to Resolve code blocks into variables
	var_t *var_deps = (var_t*)code_deps;
	for(size_t i = 0; i < deplen; i++){
		mcode_t cd_dep = code_deps[i];
		var_t vr_dep = NULL;
		for(var_t v = nmsp->head; v; v = v->next) if(v->code == cd_dep){
			vr_dep = v;
			break;
		}
		
		// Throw error if no variable is found
		if(!vr_dep){
			free(code_deps);
			return true;
		}
		var_deps[i] = vr_dep;  // Otherwise set variable dependency
	}
	
	// Store dependency list
	if(vr->deps) free(vr->deps);  // Remove previous list
	vr->deplen = deplen;
	vr->deps = var_deps;
	return false;
}

// Create variable with given name but with no expression
var_t nmsp_put(namespace_t nmsp, const char *key, size_t keylen){
	// Return if there already is a variable with that name
	if(nmsp_get(nmsp, key, keylen)) return NULL;
	
	// Allocate new code block for variable
	mcode_t code = mcode_new(-1, 8);
	return place_var_unsafe(nmsp, key, keylen, code, false);
}



// Methods used by nmsp_define
// ----------------------------

// Find circular dependency
static bool find_circ(namespace_t nmsp, var_t start){
	if(!nmsp || !start) return false;
	
	// Clear out any previous dependency tree
	nmsp->circ_root = NULL;
	for(var_t v = nmsp->head; v; v = v->next) v->used_by = NULL;
	
	// Initialize with start's immediate dependencies
	struct queue_s q = queue_new(8);
	queue_push(&q, (void**)start->deps, start->deplen);
	// Set their reference to `start`
	for(size_t i = 0; i < start->deplen; i++) start->deps[i]->used_by = start;
	
	// Iterate over variables checking their dependencies
	while(q.len > 0){  // While there are remaining variables to check
		// Get variable
		var_t vr = queue_pop(&q);
		
		// Check if it matches the root variable
		if(vr == start){
			nmsp->circ_root = start;
			queue_free(q);  // Cleanup queue
			return true;  // Circular dependency found
		}
		
		// If variable's expression has no variables go to next
		if(!vr->deps || vr->deplen == 0) continue;
		
		var_t *deps = vr->deps;
		size_t deplen = vr->deplen;
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
static size_t parse_label(const char *str, const char **endptr, const char **args, size_t *arity){
	if(endptr) *endptr = str;
	if(arity) *arity = 0;
	if(args) *args = NULL;
	
	// Collect label characters
	const char *lbl = str;
	while(isalnum(*str) || *str == '_') str++;
	size_t lbl_len = str - lbl;
	if(lbl_len == 0) return 0;
	
	while(isblank(*str)) str++;  // Skip blankspace after label
	
	size_t argcnt = 0;
	if(*str == '('){
		if(args) *args = str + 1;
		
		// Validate argument list
		do{
			str++;
			while(isspace(*str)) str++;  // Skip whitespace before argument
			if(!isalnum(*str) && *str != '_') return 0;  // Check for alphanumeric
			while(isalnum(*str) || *str == '_') str++;  // Skip name
			while(isspace(*str)) str++;  // Skip whitespace after argument
			
			argcnt++;  // Count argument
		}while(*str == ',');
		if(*str != ')') return 0;  // Check for parenthesis at the end
		str++;  // Consume ')'
		while(isblank(*str)) str++;  // Skip blankspace after args
	}
	
	// Check for colon after label
	if(*str != ':') return 0;
	str++;  // Consume ':'
	
	if(endptr) *endptr = str;
	if(arity) *arity = argcnt;
	return lbl_len;
}

var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, nmsp_err_t *errp){
	// Parse Label
	// ------------
	// Collect label characters
	while(isblank(*str)) str++;  // Skip whitespace before
	const char *lbl = str;
	
	// Try to parse label
	const char *args = NULL;
	size_t arity;
	size_t lbl_len = parse_label(str, &str, &args, &arity);
	
	if(lbl_len == 0){  // No Label was parsable
		str = lbl;
		lbl = NULL;
		args = NULL;
		arity = 0;
	}
	
	// Make sure errp points to something
	nmsp_err_t tmperr;
	if(!errp) errp = &tmperr;
	*errp = NMSP_ERR_OK;  // Set error to successful
	
	// Check for Existing Variable
	// ----------------------------
	var_t oldvar = nmsp_get(nmsp, lbl, lbl_len);
	mcode_t code = NULL;
	if(oldvar){
		if(oldvar->has_impl){  // Check for redefinition
			*errp = INSERT_ERR_REDEF;
			nmsp->redef = oldvar;
			return NULL;
		}
		
		if(mcode_get_arity(oldvar->code) != arity){  // Check for matching arity
			*errp = NMSP_ERR_ARITY_MISMATCH;
			return NULL;
		}
		
		code = oldvar->code;  // Use old code block
	}
	// Create new code block if none present
	if(!code) code = mcode_new(arity, 8);
	
	// Parse Expression
	// -----------------
	*errp = mcode_parse(code, str, endptr, args, nmsp);
	if(*errp) return NULL;  // On Parse Error
	
	
	// Insert Expression
	// ------------------
	if(oldvar){
		oldvar->has_impl = true;
		var_calc_deps(nmsp, oldvar);  // Calculate dependencies of variable
		
		if(find_circ(nmsp, oldvar)){  // Check for circular dependency
			mcode_reset(oldvar->code);  // Reset code block
			oldvar->has_impl = false;
			*errp = INSERT_ERR_CIRC;
			return NULL;
		}
		return oldvar;
	
	// Create new variable
	}else return place_var_unsafe(nmsp, lbl, lbl_len, code, true);
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









// Place builtin on stack from bltn
static void push_bltn(oper_stack_t *opstk, bltn_t bltn){
	struct stk_oper_s elem;
	elem.is_code = false;
	elem.is_block = false;
	elem.is_oper = false;
	elem.prec_assoc = 0;
	elem.bltn = bltn;
	vecpush(*opstk, elem);
}

static void push_oper(oper_stack_t *opstk, bltn_oper_t oper){
	struct stk_oper_s elem;
	elem.is_code = false;
	elem.is_block = false;
	elem.is_oper = true;
	elem.prec_assoc = (oper->prec << 1) | oper->assoc | !!oper->is_unary;
	elem.oper = oper;
	vecpush(*opstk, elem);
}

// Place comma or open parenthesis on operator stack
static void push_block(oper_stack_t *opstk, bool is_comma){
	struct stk_oper_s elem;
	elem.is_code = false;
	elem.is_block = true;
	elem.is_oper = false;
	elem.is_comma = is_comma;
	vecpush(*opstk, elem);
}

// Place a user-defined function call on the stack
static void push_call(oper_stack_t *opstk, mcode_t code){
	struct stk_oper_s elem;
	elem.is_code = true;
	elem.is_block = false;
	elem.is_oper = false;
	elem.prec_assoc = 0;
	elem.code = code;
	vecpush(*opstk, elem);
}


static nmsp_err_t apply_oper(mcode_t code, struct stk_oper_s elem){
	bool is_err;
	if(elem.is_code){
		is_err = mcode_call_code(code, elem.code);
	}else if(elem.is_oper){
		is_err = mcode_call_func(code,
			1 + !(elem.oper->is_unary),
			elem.oper->func,
			nmsp_eval_on_parse
		);
	}else{
		is_err = mcode_call_func(code,
			elem.bltn->arity,
			elem.bltn->func,
			nmsp_eval_on_parse
		);
	}
	
	if(is_err) return NMSP_ERR_MISSING_VALUES;
	return NMSP_ERR_OK;
}

// Place `op` onto `opstk` displacing operators as necessary and applying them to `exp`
static nmsp_err_t displace_opers(mcode_t code, oper_stack_t *opstk, int8_t prec){
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
		// Try to add operator onto code
		nmsp_err_t err = apply_oper(code, *elem);
		if(err) return err;
	}
	
	return NMSP_ERR_OK;
}

// Clears out parenthetical block from operator stack
// And calls function if necessary
static nmsp_err_t close_parenth(mcode_t code, oper_stack_t *opstk){
	// Remove all operators until close parenthesis
	nmsp_err_t err;
	if(err = displace_opers(code, opstk, -1)) return err;
	
	// Consume commas to find number of values in parenthetical block
	size_t arity = 1;
	struct stk_oper_s *elem;
	while((elem = veclast(*opstk)) && elem->is_block && elem->is_comma){
		vecpop(*opstk);  arity++;
	}
	
	// Check for open parenthesis
	if(!(elem = veclast(*opstk)) || !elem->is_block){
		return NMSP_ERR_PARENTH_MISMATCH;  // No opening parenthesis so parenth mismatch
	}
	vecpop(*opstk);  // Remove open parenthesis
	
	// Check for function below open parenthesis
	if((elem = veclast(*opstk))
	&& !elem->is_block && !elem->is_oper
	){  // Treat parenthetical block as arguments to function
		vecpop(*opstk);  // Remove function from `opstk`
		if(elem->is_code){
			mcode_t callee = elem->code;
			// If variable hasn't been initialized set arity
			mcode_set_arity(callee, arity);
			// Check that arity matches
			if(arity != mcode_get_arity(callee)) return NMSP_ERR_ARITY_MISMATCH;
			
			mcode_call_code(code, callee);  // Append call to `callee` onto `code`
		}else{
			bltn_t bltn = elem->bltn;
			if(arity != bltn->arity) return NMSP_ERR_ARITY_MISMATCH;  // Check that arity matches
			
			// Append call to `bltn` onto `code`
			mcode_call_func(code, bltn->arity, bltn->func, nmsp_eval_on_parse);
		}
		
	}else{  // Treat parenthetical block as value
		if(arity > 1) return NMSP_ERR_BAD_COMMA;  // Arity must be 1
	}
	
	return NMSP_ERR_OK;
}

// Try to parse sequence of alphanumerics into argument name
static int parse_arg(const char *name, size_t namelen, const char *args){
	if(!args) return -1;
	
	int argidx = 0;
	do{
		while(isspace(*args)) args++;  // Skip leading whitespace
		
		// Check for match
		bool is_match = true;
		int i = 0;
		while(isalnum(*args) || *args == '_'){
			if(is_match){
				// Check if argument and name continue to match
				if(i >= namelen || *args != name[i]) is_match = false;
				else i++;
			}
			args++;
		}
		
		// On match leave
		if(is_match && (i == namelen || name[i] == '\0')) return argidx;
		
		while(isspace(*args)) args++;  // Skip trailing whitespace
		argidx++;
	}while(*(args++) == ',');
	
	return -1;
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
static nmsp_err_t mcode_parse(mcode_t code, const char *str, const char **endptr, const char *args, namespace_t nmsp){
	// Initialize operator stack
	oper_stack_t opstk;
	vecinit(opstk, 8);
	
	nmsp_err_t err = NMSP_ERR_OK;  // Store any parse errors
	
	bool was_last_val = false;  // Track if the last token parsed was a constant or variable
	bool was_last_block = false;  // Track is last token was parenth or comma
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
			push_block(&opstk, false);  // Place open parenthesis on operator stack
			was_last_val = false;  // New expression so there is no last value
			was_last_block = true;
			continue;
		}else if(c == ','){
			if(parenth_depth == 0){  // Comma must be inside parentheses
				err = NMSP_ERR_BAD_COMMA;
				break;
			}
			str++;  // Consume ','
			if(err = displace_opers(code, &opstk, -1)) break;  // Displace all operators until block
			
			push_block(&opstk, true);  // Place comma on operator stack
			was_last_val = false;  // New expression so there is no last value
			was_last_block = true;
			continue;
		}else if(c == ')'){
			if(was_last_block){
				err = NMSP_ERR_MISSING_VALUES;
				break;
			}
			
			str++;  // Consume ')'
			parenth_depth--;
			if(err = close_parenth(code, &opstk)) break;
			was_last_val = true;
			continue;
		}
		was_last_block = false;
		
		// Try to parse operator
		bltn_oper_t oper = bltn_oper_parse(str, &after_tok, !was_last_val);
		if(oper && after_tok > str){
			struct stk_oper_s *last = veclast(opstk);
			if(last && !last->is_block){  // For non-block previous operator
				// Check that last operator isn't function
				if(!last->is_oper){ err = NMSP_ERR_FUNC_NOCALL;  break; }
				
				// When operator is unary
				// Check that previous operator is a unary, right-associative, or lower precedence
				bltn_oper_t lst_oper = last->oper;
				if(oper->is_unary == 1 && !(lst_oper->is_unary
				|| lst_oper->assoc == OPER_RIGHT_ASSOC
				|| lst_oper->prec < oper->prec
				// Otherwise we have a unary operator conflicting with a binary operator
				)){ err = NMSP_ERR_LOWPREC_UNARY;  break; }
			}
			
			
			// Only displace operators if binary operator
			if(!oper->is_unary){  // If operator follows val it is binary
				if(err = displace_opers(code, &opstk, (int8_t)(oper->prec))) break;
			}
			push_oper(&opstk, oper);  // Place operator at top of stack
			str = after_tok;  // Move string forward
			was_last_val = false;  // Operator is not a value
			continue;
		}
		
		
		// Try to parse constant
		arith_t val = arith_parse(str, &after_tok);
		if(val){
			// Cannot have two values in a row
			if(was_last_val){ err = NMSP_ERR_MISSING_OPERS;  break; }
			
			mcode_load_const(code, val);  // Load constant into code block
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
		int argind = parse_arg(str, namelen, args);
		if(argind >= 0){
			// Cannot have two values in a row
			if(was_last_val){ err = NMSP_ERR_MISSING_OPERS;  break; }
			
			mcode_load_arg(code, argind);
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Try to parse builtin function or constant name
		bltn_t bltn = bltn_parse(str, namelen);
		if(bltn){
			// Cannot have two values in a row
			if(was_last_val){ err = NMSP_ERR_MISSING_OPERS;  break; }
			
			if(bltn->arity == 0){  // Place constant on value stack (i.e. expression)
				mcode_load_const(code, bltn->func(NULL, NULL));
			}else{  // Place function on operator stack
				push_bltn(&opstk, bltn);
			}
			str = after_tok;
			was_last_val = true;
			continue;
		}
		
		// Try to parse variable name
		// Collect alphanumerics and _
		var_t vr = parse_var(str, namelen, nmsp);
		if(vr){
			// Cannot have two values in a row
			if(was_last_val){ err = NMSP_ERR_MISSING_OPERS;  break; }
			
			// Check if next character is '('
			const char *tmp = after_tok;
			while(parenth_depth > 0 ? isspace(*tmp) : isblank(*tmp)) tmp++;
			if(*tmp == '('){  // Treat `vr` as a user-defined function
				// If already defined check that `vr` is a function
				if(mcode_get_arity(vr->code) == 0){ err = NMSP_ERR_MISSING_OPERS;  break; }
				push_call(&opstk, vr->code);
			}else{  // Treat `vr` as a variable
				mcode_set_arity(vr->code, 0);  // Variable has no args
				mcode_call_code(code, vr->code);  // Load variable into code
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
	if(err){
		vecfree(opstk);  // Deallocate operator stack
		return err;
	}
	
	// Clear out remaining operators on the operator stack
	struct stk_oper_s op;
	while(!vecempty(opstk)){
		op = vecpop(opstk);
		// Make sure no open parenths are left
		if(op.is_block){ err = NMSP_ERR_PARENTH_MISMATCH;  break; }
		// Or function calls
		if(!op.is_oper){ err = NMSP_ERR_FUNC_NOCALL;  break; }
		
		// Apply operator to expression
		if(err = apply_oper(code, op)) break;
	}
	vecfree(opstk);  // Deallocate stack
	
	return err;
}

