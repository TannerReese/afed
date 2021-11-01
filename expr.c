#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#include "expr.h"


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
#define INSTR_LOAD(exp, inst) (INSTR_IS_VAR(inst) ? (exp)->vars[INSTR_LOAD_INDEX(inst)]->cached : (exp)->consts[INSTR_LOAD_INDEX(inst)])

struct expr_s {
	// Outside variables loaded at runtime
	size_t varlen, varcap;
	var_t *vars;
	
	// Constants & Literals
	size_t constlen, constcap;  // Number of members and maximum number possible
	void **consts;
	
	// Instructions to Run
	size_t instrlen, instrcap;
	instr_t *instrs;
};




typedef uint32_t hash_t;

struct var_s {
	// Expression used to calculate the value of this variable
	expr_t expr;
	
	// Cached value of calculation
	// NULL if no value is cached
	void *cached;
	// Error that occurred when calculating cached
	expr_err_t err;
	
	// Name of variable
	size_t namelen;
	const char *name;
	hash_t hash;  // 32-bit hash of name
	
	// Allow variables to be structured in linked list
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

// Get value of variable
void *nmsp_var_value(var_t vr, expr_err_t *err){
	// Calculate the value if not cached
	if(!vr->cached) vr->cached = expr_eval(vr->expr, &(vr->err));
	
	if(err) *err = vr->err;
	return vr->cached;
}



struct namespace_s {
	// Number of variables currently stored in the linked list
	size_t length;
	
	/* Used by dependency checker
	 *  `circ_root` is a variable which depends
	 *  on itself through a series of variables
	 */
	var_t circ_root;
	
	// Head of linked list
	struct var_s *head;
};

// Create new empty namespace
namespace_t nmsp_new(){
	namespace_t nmsp = malloc(sizeof(struct namespace_s));
	nmsp->length = 0;
	nmsp->circ_root = NULL;
	nmsp->head = NULL;
	return nmsp;
}

// Deallocate namespace, its variables, and their expressions
void nmsp_free(namespace_t nmsp){
	// Deallocate variables of namespace
	var_t next, curr = nmsp->head;
	while(curr){
		next = curr->next;
		// Deallocate variable members
		if(curr->expr) expr_free(curr->expr);
		if(curr->cached) expr_valctl.free(curr->cached);
		free(curr);
		curr = next;
	}
	
	// Deallocate namespace itself
	free(nmsp);
}

// Get instance of variable using name
var_t nmsp_get(namespace_t nmsp, const char *key, size_t keylen){
	hash_t keyhash = hash(key, keylen);
	for(var_t vr = nmsp->head; vr; vr = vr->next){
		if(vr->hash == keyhash  // Check for matching hash (should filter out most time)
		&& vr->namelen == keylen  // Check for same length
		&& strncmp(vr->name, key, keylen) == 0)  // Finally perform string comparison
			return vr;
		// If we have passed the point where the hash would be then leave
		else if(vr->hash > keyhash) return NULL;
	}
	return NULL;
}



// Place new variable in namespace
// WARNING: Does not perform any checks for existence or dependency
static var_t place_var_unsafe(namespace_t nmsp, const char *key, size_t keylen, expr_t exp){
	// Allocate memory for new variable
	var_t newvr = malloc(sizeof(struct var_s));
	newvr->expr = exp;
	newvr->cached = NULL;
	
	// Allocate space for name
	newvr->namelen = keylen;
	char *nm = malloc(keylen * sizeof(char));
	strncpy(nm, key, keylen);
	newvr->name = nm;
	hash_t keyhash = hash(key, keylen);
	newvr->hash = keyhash;
	
	// Set used_by pointer to NULL as default
	newvr->used_by = NULL;
	
	// Find location to put into namespace
	if(!nmsp->head || nmsp->head->hash >= keyhash){  // Check if it belongs at beginning
		newvr->next = nmsp->head;
		nmsp->head = newvr;
		
	}else{
		var_t vr;
		// Loop until you find the first hash greater than the current one
		for(vr = nmsp->head; vr->next && vr->next->hash < keyhash; vr = vr->next);
		newvr->next = vr->next;
		vr->next = newvr;
	}
	
	return newvr;
}

// Create variable with given name but with no expression
var_t nmsp_put(namespace_t nmsp, const char *key, size_t keylen){
	// Return if there already is a variable with that name
	if(nmsp_get(nmsp, key, keylen)) return NULL;
	return place_var_unsafe(nmsp, key, keylen, NULL);
}


// Try to insert an expression with the given name
// If the expression already exists or has a circular dependency returns NULL
// If there is a circular dependency the dependency list will be set
var_t nmsp_insert(namespace_t nmsp, const char *key, size_t keylen, expr_t exp){
	// Check for a forward declaration for this expression
	var_t newvr;
	if(newvr = nmsp_get(nmsp, key, keylen)){
		if(newvr->expr){
			// Check if variable already has an expression
			// Error: key is already defined
			nmsp->circ_root = NULL;
			return NULL;
		}
		
		
		// Check for circular dependency
		// Only necessary if there was a forward declaration
		
		// Create queue of variables
		size_t qstart = 0, qlen = exp->varlen, qcap = exp->varlen << 1;
		// Initially provide enough space to store all of exp's variables
		var_t *queue = malloc(qcap * sizeof(var_t));
		
		// Clear out any previous dependency tree
		nmsp->circ_root = NULL;
		for(var_t v = nmsp->head; v; v = v->next) v->used_by = NULL;
		
		// Initialize with exp's immediate dependencies
		memcpy(queue, exp->vars, qlen * sizeof(var_t));
		// Set their reference to `newvr`
		for(size_t i = 0; i < qlen; i++) queue[i]->used_by = newvr;
		
		// Iterate over variables checking their dependencies
		while(qlen > 0){  // While there are remaining variables to check
			// Get variable
			var_t vr = queue[qstart++];  qlen--;
			qstart -= qcap & -(size_t)(qstart >= qcap);  // Shift qstart down if it passes the max
			
			// Check if it matches the root variable
			if(newvr == vr){
				nmsp->circ_root = vr;
				free(queue);  // Cleanup queue
				return NULL;
			}
			
			
			// Resize queue to accomodate variables used by `vr`
			size_t qend = qstart + qlen;  // Find end of queue
			qend -= qcap & -(size_t)(qend >= qcap);
			if(qlen + vr->expr->varlen > qcap){  // Resize queue if necessary
				size_t oldcap = qcap;
				while(qlen + vr->expr->varlen >= qcap) qcap <<= 1;
				queue = realloc(queue, qcap);
				
				if(qend <= qstart){
					// Move any discontiguous piece together
					memcpy(queue + oldcap, queue, qend * sizeof(var_t));
					qend = qstart + qlen;
				}
			}
			
			// Add all the variables used by `vr` to the queue
			qlen += vr->expr->varlen;
			for(size_t i = 0; i < vr->expr->varlen; i++){
				var_t dep = vr->expr->vars[i];
				dep->used_by = vr;
				
				queue[qend++] = dep;  // Add to queue
				qend -= qcap & -(size_t)(qend >= qcap);  // shift qend down if it passes the max
			}
		}
		
		// If no circular dependency
		newvr->expr = exp;  // Set expression
		return newvr;
		
	}else{  // If no forward declared variable then create new variable
		hash_t keyhash = hash(key, keylen);
		return place_var_unsafe(nmsp, key, keylen, exp);
	}
}

// Get next variable in dependency chain starting from base of circular dependency
var_t nmsp_next_dep(namespace_t nmsp){
	var_t vr = nmsp->circ_root;
	if(nmsp->circ_root) nmsp->circ_root = nmsp->circ_root->used_by;
	return vr;
}




// Allocate empty expression
expr_t expr_new(size_t varcap, size_t constcap, size_t instrcap){
	expr_t exp = malloc(sizeof(struct expr_s));
	exp->varlen = 0;  exp->varcap = varcap;
	exp->vars = malloc(varcap * sizeof(var_t));
	exp->constlen = 0;  exp->constcap = constcap;
	exp->consts = malloc(constcap * sizeof(void*));
	exp->instrlen = 0;  exp->instrcap = instrcap;
	exp->instrs = malloc(instrcap * sizeof(instr_t));
	return exp;
}

// Deallocate expression
void expr_free(expr_t exp){
	free(exp->instrs);
	
	// Deallocate any stored constants
	for(size_t i = 0; i < exp->constlen; i++) free(exp->consts[i]);
	free(exp->consts);
	
	free(exp->vars);
	free(exp);
}

expr_t expr_new_var(var_t vr){
	expr_t exp = malloc(sizeof(struct expr_s));
	exp->varlen = 1;  exp->varcap = 2;
	exp->vars = malloc(exp->varcap * sizeof(var_t));
	exp->vars[0] = vr;
	
	exp->constlen = 0;  exp->constcap = 0;
	exp->consts = NULL;
	
	exp->instrlen = 1;  exp->instrcap = 2;
	exp->instrs = malloc(exp->instrcap * sizeof(instr_t));
	exp->instrs[0] = INSTR_NEW_VAR(0);
	return exp;
}

expr_t expr_new_const(void *val){
	expr_t exp = malloc(sizeof(struct expr_s));
	exp->varlen = 0;  exp->varcap = 0;
	exp->vars = NULL;
	
	exp->constlen = 1;  exp->constcap = 2;
	exp->consts = malloc(exp->constcap * sizeof(void*));
	exp->consts[0] = val;
	
	exp->instrlen = 1;  exp->instrcap = 2;
	exp->instrs = malloc(exp->instrcap * sizeof(instr_t));
	exp->instrs[0] = INSTR_NEW_CONST(0);
	return exp;
}

// Evaluate `exp` using the provided stack
static expr_err_t expr_eval_stk(expr_t exp, void **stack, void **stkend);

void *expr_eval(expr_t exp, expr_err_t *err){
	void *stack[EXPR_EVAL_STACK_SIZE];  // Allocate Stack for evaluation
	// On evaluation error return
	expr_err_t tmperr;
	if(tmperr = expr_eval_stk(exp, stack, stack + EXPR_EVAL_STACK_SIZE)){
		if(err) *err = tmperr;
		return NULL;
	}
	
	// Return bottom of stack
	return *stack;
}

// Evaluate `exp` using the provided stack
static expr_err_t expr_eval_stk(expr_t exp, void **stack, void **stkend){
	// If no expression is provided
	if(!exp) return EVAL_ERR_NO_EXPR;
	
	expr_err_t err = EVAL_ERR_OK;  // Track any error that happen
	
	// Evaluate any variable dependencies
	for(size_t i = 0; i < exp->varlen; i++){
		var_t vr = exp->vars[i];
		
		// Evaluate expression if no cached variable
		if(!vr->cached){
			if(err = expr_eval_stk(vr->expr, stack, stkend)) return err;
			vr->cached = *stack;  // Store evaluated value in variable
		}
	}
	
	// Store location on stack
	void** stktop = stack;
	
	// Run Instructions
	int no_free = 1;  // If the top element doesn't need to be freed on binary operation
	for(size_t i = 0; i < exp->instrlen; i++){
		instr_t inst = exp->instrs[i];
		
		if(INSTR_IS_OPER(inst)){  // Operator
			// Check that there are enough arguments
			if(stktop - !INSTR_IS_UNARY(inst) <= stack){
				err = EVAL_ERR_STACK_UNDERFLOW;
				break;
			}
			
			// Get top argument and get operator id
			void **arg = stktop - 1;
			oper_t op = INSTR_OPERID(inst);
			
			if(INSTR_IS_UNARY(inst)){  // Unary Operator
				// Perform Unary Operation
				if(err = expr_opers[op].func.unary(*arg)) break;
			}else{  // Binary Operator
				// Perform Binary Operation
				if(err = expr_opers[op].func.binary(*(arg - 1), *arg)) break;
				
				if(no_free) no_free = 1;  // Clear no_free flag
				else expr_valctl.free(*arg);  // Deallocate top element
				
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
				no_free = 1;
			}else{
				// Otherwise a clone is necessary
				val = expr_valctl.clone(val);
			}
			
			// Place value on top of stack
			*(stktop++) = val;
		}
	}
	
	// Should only be one remaining value after evaluation
	if(!err && stktop - 1 > stack){
		err = EVAL_ERR_STACK_SURPLUS;
	}
	
	if(err){
		// Cleanup values on stack
		if(no_free) stktop--;  // Ignore top value if no_free true
		for(; stktop >= stack; stktop--) expr_valctl.free(*stktop);
		return err;
	}else{
		// There will be one value left at the bottom of the stack
		return EVAL_ERR_OK;
	}
}



// Macro to increase array size enough for one element
// Does not modify the length only the capacity
#define inc_size(arr, type, cap, len) if((len) >= (cap)){ \
	/* Make sure capacity is at least one */ (cap) += (cap) == 0; \
	do{ (cap) <<= 1; }while((len) >= (cap)); \
	(arr) = realloc((arr), (cap) * sizeof(type)); \
}

// Place variable in expr variable section if not alreayd present
// Return the variable index
static size_t expr_put_var(expr_t exp, var_t vr){
	// Check if variable already present
	for(size_t i = 0; i < exp->varlen; i++) if(exp->vars[i] == vr) return i;
	
	// Add variable to variable list
	// Resize variable section if necessary
	inc_size(exp->vars, var_t, exp->varcap, exp->varlen);
	exp->vars[exp->varlen++] = vr;
	return exp->varlen - 1;
}

// Place constant value in expr constants section if not already present
// Return the constant index
static size_t expr_put_const(expr_t exp, void *val){
	// Check if constant value is already present
	for(size_t i = 0; i < exp->constlen; i++) if(exp->consts[i] == val) return i;
	
	// Add value to constants list
	// Resize constant section if necessary
	inc_size(exp->consts, void*, exp->constcap, exp->constlen);
	exp->consts[exp->constlen++] = val;
	return exp->constlen - 1;
}

// Combine `vr` onto expression `exp` using binary operator `op`
expr_t expr_binary_var(expr_t exp, var_t vr, oper_t op){
	// Put variable into expression
	size_t varid = expr_put_var(exp, vr);
	
	// Resize if necessary to hold two new instructions
	exp->instrlen++;
	inc_size(exp->instrs, instr_t, exp->instrcap, exp->instrlen);
	
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
	inc_size(exp->instrs, instr_t, exp->instrcap, exp->instrlen);
	
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
	for(size_t i = 0; i < src->constlen; i++) constmap[i] = expr_put_const(dest, src->consts[i]);
	
	
	// Iterate over src instructions to substitute variable and constant references
	for(size_t i = 0; i < src->instrlen; i++){
		instr_t inst = src->instrs[i];
		
		// Replace load index of var or const load instruction
		if(INSTR_IS_VAR(inst))
			inst = INSTR_NEW_VAR(varmap[INSTR_LOAD_INDEX(inst)]);
		else if(INSTR_IS_CONST(inst))
			inst = INSTR_NEW_CONST(constmap[INSTR_LOAD_INDEX(inst)]);
		
		inc_size(dest->instrs, instr_t, dest->instrcap, dest->instrlen);  // Resize array if necessary
		dest->instrs[dest->instrlen++] = inst;  // Put instruction into dest instrs
	}
	
	// Place operator at end of instructions
	inc_size(dest->instrs, instr_t, dest->instrcap, dest->instrlen);  // Resize array if necessary
	dest->instrs[dest->instrlen++] = INSTR_NEW_OPER(op, 0);
	
	return dest;
}

// Modify `exp` by applying unary operator `op`
expr_t expr_unary(expr_t exp, oper_t op){
	// Resize instruction array if necessary
	inc_size(exp->instrs, instr_t, exp->instrcap, exp->instrlen);
	
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
	
	return EVAL_ERR_OK;
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
static oper_t parse_oper(const char *str, const char **endptr, int is_unary){
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

// Apply single operator to value stack by appending
static void apply_oper(expr_t exp, oper_t opid){
	// Resize if necessary
	inc_size(exp->instrs, instr_t, exp->instrcap, exp->instrlen);
	exp->instrs[exp->instrlen++] = INSTR_NEW_OPER(opid, expr_opers[opid].is_unary);
}


// Place `op` onto `opstk` displacing operators as necessary and applying them to `exp`
static void displace_opers(expr_t exp, oper_stack_t *opstk, int8_t prec){
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
		apply_oper(exp, elem.operid);
	}
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

expr_t expr_parse(const char *str, const char **endptr, namespace_t nmsp, expr_err_t *err){
	// Initialize operator stack
	oper_stack_t opstk;
	opstk.cap = 256;  opstk.len = 0;
	opstk.bottom = malloc(opstk.cap * sizeof(struct stk_oper_s));
	
	// Initialize expression
	expr_t exp = expr_new(2, 2, 4);
	
	// Give err reference to prevent null dereferences
	expr_err_t tmperr;
	if(!err) err = &tmperr;
	*err = EVAL_ERR_OK;
	
	// Track if the last token parsed was a constant or variable
	int was_last_val = 0;
	while(*str){	
		// Skip whitespace
		while(isspace(*str)) str++;
		
		int c = *str;
		const char *after_tok = str;  // Pointer to after parsed token
		
		// Check for parentheses
		if(c == '('){
			str++;  // Consume '('
			// Place open parenthesis on operator stack			
			inc_size(opstk.bottom, struct stk_oper_s, opstk.cap, opstk.len);
			opstk.bottom[opstk.len++].is_block = 1;  // Indicate that it is a block
			was_last_val = 0;  // New expression so there is no last value
			continue;
		}else if(c == ')'){
			str++;  // Consume ')'
			// Remove all operators until block
			displace_opers(exp, &opstk, -1);
			// Check for block (i.e. open parenthesis)
			if(opstk.len > 0 && stktop(opstk).is_block){
				// Remove block and continue parsing
				opstk.len--;
				// Treat closed parenthetical expression as value
				was_last_val = 1;
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
			if(was_last_val) displace_opers(exp, &opstk, (int8_t)(info.prec));
			else if(!stktop(opstk).is_block){  // When unary and previous element isn't block
				// Check that previous operator is unary, left-associative, or lower precedence
				struct oper_info_s info2 = expr_opers[stktop(opstk).operid];
				if(!(info2.is_unary || info2.assoc == OPER_RIGHT_ASSOC || info2.prec < info.prec)){
					// Otherwise we have a unary operator coming after a binary operator of higher precedence
					*err = PARSE_ERR_LOWPREC_UNARY;
					break;
				}
			}
			
			// Place operator at top of stack
			inc_size(opstk.bottom, struct stk_oper_s, opstk.cap, opstk.len);
			elem.is_block = 0;
			elem.prec_assoc = (info.prec << 1) | info.assoc | info.is_unary;
			elem.operid = opid;
			opstk.bottom[opstk.len++] = elem;
			
			was_last_val = 0;  // Operator is not a value
			str = after_tok;  // Move string forward
			continue;
		}
		
		
		// Try to parse value
		void *val;
		if(val = expr_valctl.parse(str, &after_tok)){
			// Cannot have two values in a row
			if(was_last_val){
				*err = PARSE_ERR_MISSING_OPERS;
				break;
			}
			
			// Put constant into expression
			size_t constid = expr_put_const(exp, val);
			// Place value onto instruction list as constant
			inc_size(exp->instrs, instr_t, exp->instrcap, exp->instrlen);
			exp->instrs[exp->instrlen++] = INSTR_NEW_CONST(constid);
			str = after_tok;
			was_last_val = 1;
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
			inc_size(exp->instrs, instr_t, exp->instrcap, exp->instrlen);
			exp->instrs[exp->instrlen++] = INSTR_NEW_VAR(varid);
			str = after_tok;
			was_last_val = 1;
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
			// Cleanup stack and expression on error
			expr_free(exp);
			free(opstk.bottom);
			return NULL;
		}
		
		// Apply operator to expression
		apply_oper(exp, op.operid);
	}
	// Deallocate stack
	free(opstk.bottom);
	
	// Check that expression will evaluate appropriately
	if(*err = check_valid(exp)){
		// Shift error code down to be more descriptive
		*err += PARSE_ERR_TOO_MANY_VALUES - EVAL_ERR_STACK_OVERFLOW;
		expr_free(exp);
		return NULL;
	}
	
	return exp;
}
