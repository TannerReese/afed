#ifndef __PARSE_TREE_H
#define __PARSE_TREE_H

#include <stdbool.h>

#define PARSE_TREE_CHAR_MAX 8

/* Store a collection of words associated with IDs
 * as a tree in which each word has a node.
 * 
 * If word A is a prefix of word B
 * then word B will be a descendant node of A
 * Allows for longest prefix matching
 */
struct ptree_s;
typedef struct ptree_s *ptree_t;

#define ptree_new() NULL
void ptree_free(ptree_t pt);

// Add `word` to the set with ID `id` ; Returns true on success
#define ptree_put(pt, word, id) ptree_putn(pt, word, -1, id)
// If `n` >= 0 then only use first `n` characters of `word`
bool ptree_putn(ptree_t *pt, const char *word, int n, int id);

// Match longest prefix of `str` contained in `pt`
// Return negative if none are found
#define ptree_get(pt, str, endptr) ptree_getn(pt, str, -1, endptr)
// If `n` >= 0 then only use first `n` characters of `word`
int ptree_getn(ptree_t pt, const char *str, int n, const char **endptr);

#endif

