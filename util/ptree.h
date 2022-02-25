#ifndef __PARSE_TREE_H
#define __PARSE_TREE_H

#include <stdbool.h>

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

// Add `word` to the set with target `tgt` ; Returns true on success
#define ptree_put(pt, word, val) ptree_putn(pt, word, -1, val)
// If `n` >= 0 then only use first `n` characters of `word`
bool ptree_putn(ptree_t *pt, const char *word, int n, void *tgt);

// Match longest prefix of `str` contained in `pt`
// Return NULL if none are found
#define ptree_get(pt, str, endptr) ptree_getn(pt, str, -1, endptr)
// If `n` >= 0 then only use first `n` characters of `word`
void *ptree_getn(ptree_t pt, const char *str, int n, const char **endptr);

#endif

