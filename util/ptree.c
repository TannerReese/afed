#include <stdlib.h>

#include "ptree.h"

// Store set of words and find longest prefix match
struct ptree_s {
	// Last character of word
	char c;
	
	/* If non-NULL then `target` is the target
	 * of the word associated with this node
	 * If NULL then this node has no word
	 */
	void *target;
	
	struct ptree_s *next;  // Pointer to next sibling node
	struct ptree_s *child;  // Pointer to first child node
};


void ptree_free(ptree_t pt){
	while(pt){
		ptree_free(pt->child);  // Free children of node
		
		ptree_t next = pt->next;
		free(pt);  // Free node itself
		pt = next;
	}
}

bool ptree_putn(ptree_t *pt, const char *word, int n, void *tgt){
	if(!*word) return false;
	
	ptree_t *loc = pt;
	
	// Create or find node for each letter in word
	const char *end = word + n;
	for(; *word && (n < 0 || word < end); word++){
		// Descend to child if not the root node
		if(loc != pt) loc = &((*loc)->child);
		
		// Check if current letter exists in tree
		for(; *loc; loc = &((*loc)->next)){
			if((*loc)->c == *word) break;
		}
		
		if(!*loc){  // If no node found, make new node for character
			ptree_t nd = malloc(sizeof(struct ptree_s));
			nd->c = *word;
			nd->target = NULL;
			nd->next = NULL;  nd->child = NULL;
			*loc = nd;
		}
	}
	
	// Set target of node equal to `tgt`
	(*loc)->target = tgt;
	return true;
}


void *ptree_getn(ptree_t pt, const char *str, int n, const char **endptr){
	const char *end = str + n;
	void *tgt = NULL;  // Track target of current longest prefix
	for(; *str && (n < 0 || str < end); str++){
		// Find character
		for(; pt; pt = pt->next) if(pt->c == *str) break;
		if(!pt) break;  // If no node found then leave
		
		if(pt->target){  // If node has ID set `id` to it
			tgt = pt->target;
			if(endptr) *endptr = str + 1;
		}
		pt = pt->child;  // Descend to child and continue
	}
	
	return tgt;
}


