#include <stdlib.h>

#include "ptree.h"

// Store set of words and find longest prefix match
struct ptree_s {
	// Last few characters of word
	char value;
	
	/* If positive (or 0) then `id` is the ID
	 * of the word associated with this node
	 * If negative then this node has no word
	 */
	int id;
	
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

bool ptree_putn(ptree_t *pt, const char *word, int n, int id){
	if(!*word) return false;
	
	ptree_t *loc = pt;
	
	// Create or find node for each letter in word
	const char *end = word + n;
	for(; *word && (n < 0 || word < end); word++){
		// Descend to child if not the root node
		if(loc != pt) loc = &((*loc)->child);
		
		// Check if current letter exists in tree
		for(; *loc; loc = &((*loc)->next)){
			if((*loc)->value == *word) break;
		}
		
		if(!*loc){  // If no node found, make new node for character
			ptree_t nd = malloc(sizeof(struct ptree_s));
			nd->value = *word;
			nd->id = -1;
			nd->next = NULL;  nd->child = NULL;
			*loc = nd;
		}
	}
	
	// Set ID of node equal to `id`
	(*loc)->id = id;
	return true;
}


int ptree_getn(ptree_t pt, const char *str, int n, const char **endptr){
	const char *end = str + n;
	int id = -1;  // Track current longest prefix
	for(; *str && (n < 0 || str < end); str++){
		// Find character
		for(; pt; pt = pt->next) if(pt->value == *str) break;
		if(!pt) break;  // If no node found then leave
		
		if(pt->id >= 0){  // If node has ID set `id` to it
			id = pt->id;
			if(endptr) *endptr = str + 1;
		}
		pt = pt->child;  // Descend to child and continue
	}
	
	return id;
}


