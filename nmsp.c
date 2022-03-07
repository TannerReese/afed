#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>

// Utilities
#include "util/queue.h"  // Queue of variables
#include "util/mcode.h"  // Executable code blocks

#include "nmsp.h"
#include "bltn.h"



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
	
	/* If this namespace should try
	 * to simplify literals while parsing
	 */
	bool try_eval;
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

/* Check for dependencies cycles starting from `start`
 * Return true if a circular dependency is found
 */
static bool find_circ(namespace_t nmsp, var_t start);


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
static parse_err_t mcode_parse(mcode_t code, const char *str, const char **endptr, const char *args, namespace_t nmsp);





// Returns a string containing a description of errors
const char *nmsp_strerror(parse_err_t err){
	switch(err){
		case PARSE_ERR_OK: return "PARSE_ERR_OK: Successful";
		
		// Parsing Errors
		case PARSE_ERR_PARENTH_MISMATCH: return "PARSE_ERR_PARENTH_MISMATCH: Missing open or close parenthesis";
		case PARSE_ERR_LOWPREC_UNARY: return "PARSE_ERR_LOWPREC_UNARY: Unary operator follows Binary of Higher Precedence";
		case PARSE_ERR_ARITY_MISMATCH: return "PARSE_ERR_ARITY_MISMATCH: Wrong number of arguments given to function";
		case PARSE_ERR_BAD_COMMA: return "PARSE_ERR_BAD_COMMA: Comma in wrong location";
		case PARSE_ERR_VAR_CALL: return "PARSE_ERR_VAR_CALL: Variable cannot be called";
		case PARSE_ERR_FUNC_NOCALL: return "PARSE_ERR_FUNC_NOCALL: Function present but not called";
		
		// Produce after parsing produces invalid expression
		case PARSE_ERR_MISSING_VALUES: return "PARSE_ERR_MISSING_VALUES: Operator is missing argument";
		case PARSE_ERR_MISSING_OPERS: return "PARSE_ERR_MISSING_OPERS: Multiple values without operator between";
		case PARSE_ERR_EXTRA_CONT: return "PARSE_ERR_EXTRA_CONT: Values present after expression";
		
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
	if(errp) *errp = EVAL_ERR_INCOMPLETE_CODE;
	return NULL;
}

// Print variable value to a file
int nmsp_var_fprint(FILE *stream, var_t vr){
	// Calculate value
	arith_err_t err = EVAL_ERR_OK;
	void *val = mcode_eval(vr->code, NULL, &err);
	
	// Print value
	if(err) return fprintf(stream, "ERR %i", err);
	else return arith_print(stream, val);
}



/* Allocate namespace
 *  If eval_on_parse then the namespace will
 *  attempt to simplify literals while parsing.
 */
namespace_t nmsp_new(bool eval_on_parse){
	namespace_t nmsp = malloc(sizeof(struct namespace_s));
	nmsp->head = NULL;
	nmsp->redef = NULL;
	nmsp->circ_root = NULL;
	nmsp->try_eval = eval_on_parse;
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

var_t nmsp_define(namespace_t nmsp, const char *str, const char **endptr, parse_err_t *errp){
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
	parse_err_t tmperr;
	if(!errp) errp = &tmperr;
	*errp = PARSE_ERR_OK;  // Set error to successful
	
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
			*errp = PARSE_ERR_ARITY_MISMATCH;
			return NULL;
		}
		
		code = oldvar->code;  // Use old code block
		mcode_set_arity(code, arity);
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
static parse_err_t mcode_parse(mcode_t code, const char *str, const char **endptr, const char *args, namespace_t nmsp){
	shunt_t shn = shunt_new(code, nmsp->try_eval, 4);  // Initialize shunting yard
	parse_err_t err = PARSE_ERR_OK;  // Store any parse errors
	
	// Track parenthesis depth to see if newlines should be consumed
	int parenth_depth = 0;
	const char *after_tok = str;  // Pointer to after parse token
	for(; *str; str = after_tok){
		// Skip whitespace
		while(parenth_depth > 0 ? isspace(*str) : isblank(*str)) str++;
		if(parenth_depth == 0 && *str == '\n') break;  // Leave at newline outside parenthesis
		after_tok = str;
		
		int c = *str;
		if(c == '('){
			after_tok++;  // Consume '('
			parenth_depth++;
			if(err = shunt_open_parenth(shn)) break;
			continue;
		}else if(c == ','){
			if(parenth_depth == 0){  // Comma must be inside parentheses
				err = PARSE_ERR_BAD_COMMA;
				break;
			}
			after_tok++;  // Consume ','
			if(err = shunt_put_comma(shn)) break;
			continue;
		}else if(c == ')'){
			after_tok++;  // Consume ')'
			parenth_depth--;
			if(err = shunt_close_parenth(shn)) break;
			continue;
		}
		
		// Try to parse operator
		bltn_oper_t oper = bltn_oper_parse(str, &after_tok, !shunt_was_last_val(shn));
		if(oper && after_tok > str){
			if(oper->is_unary){
				if(err = shunt_put_unary(shn, oper->func, oper->prec)) break;
			}else{
				if(err = shunt_put_binary(shn, oper->func, oper->prec, oper->assoc)) break;
			}
			continue;
		}
		
		
		// Try to parse constant
		arith_t val = arith_parse(str, &after_tok);
		if(val){
			if(err = shunt_load_const(shn, val)) break;
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
			if(err = shunt_load_arg(shn, argind)) break;
			continue;
		}
		
		// Try to parse builtin function or constant name
		bltn_t bltn = bltn_parse(str, namelen);
		if(bltn){
			if(bltn->arity == 0){  // When `bltn` is a constant
				if(err = shunt_load_const(shn, bltn->func(NULL, NULL))) break;
			}else{  // When `bltn` is a function
				if(err = shunt_func_call(shn, bltn->arity, bltn->func)) break;
			}
			continue;
		}
		
		// Try to parse variable name
		// Collect alphanumerics and _
		var_t vr = parse_var(str, namelen, nmsp);
		if(vr){
			// Check if next character is '('
			const char *tmp = after_tok;
			while(parenth_depth > 0 ? isspace(*tmp) : isblank(*tmp)) tmp++;
			
			if(*tmp == '('){  // Treat `vr` as a user-defined function
				if(err = shunt_code_call(shn, vr->code)) break;
			}else{  // Treat `vr` as a variable
				if(err = shunt_load_var(shn, vr->code)) break;
			}
			continue;
		}
		
		break;  // Failed to parse token
	}
	
	// Move endpointer to after parsed section
	if(endptr) *endptr = str;
	
	if(err){  // On error cleanup and leave
		shunt_free(shn);
		return err;
	}
	
	// Clear out remaining operators on the operator stack
	if(err = shunt_clear(shn)) return err;
	shunt_free(shn);
	return PARSE_ERR_OK;
}

