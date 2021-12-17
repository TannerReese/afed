#ifndef __DOCMT_H
#define __DOCMT_H

#include "nmsp.h"
#include <stdio.h>

struct docmt_s;
typedef struct docmt_s *docmt_t;

/* Create document in heap memory
 * Which will parse string `str`
 * And store the variables in `nmsp`
 */
docmt_t docmt_new(const char *str, namespace_t nmsp);
// Return the namespace associated to the document
namespace_t docmt_get_nmsp(docmt_t doc);
/* Deallocate memory for document
 * NOT including the associated namespace
 */
void docmt_free(docmt_t doc);

// Parse statements in string producing pieces
void docmt_parse(docmt_t doc, FILE *errout);
// Print pieces to `stream`
int docmt_fprint(docmt_t doc, FILE *stream);

#endif

