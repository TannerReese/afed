#include "docmt.h"

#include <stdlib.h>
#include <stdbool.h>
#include <ctype.h>
#include <stdio.h>

// A piece that will be printed to the output file
struct piece_s {
	// Indicate if the piece is a slice or an named / unnamed value
	bool is_slice : 1;
	
	// Reference to value to be printed
	union {
		struct {
			const char *start;
			size_t length;
		} slice;
		
		var_t var;
	} source;
};

struct docmt_s {
	// Vector of pieces
	size_t pclen, pccap;
	struct piece_s *pieces;
	
	// Store variables contained in document
	namespace_t nmsp;
	
	// Remaining portion of string
	// Serves as beginning of next piece
	const char *remd;
	
	const char *str;  // Current Location of parsing
	int line_no;  // Line number within string
};

/* Create a slice from `remd` to `str`
 * And add it to the document
 * Finally, set `remd` equal to `str`
 */
static void add_slice(docmt_t doc){
	struct piece_s pc;
	pc.is_slice = true;
	
	// Set content of slice
	pc.source.slice.start = doc->remd;
	pc.source.slice.length = doc->str - doc->remd;
	// Move the remaining pointer forward
	doc->remd = doc->str;
	
	// Create space if necessary
	if(doc->pclen >= doc->pccap){
		doc->pccap <<= 1;
		doc->pieces = realloc(doc->pieces, sizeof(struct piece_s) * doc->pccap);
	}
	doc->pieces[doc->pclen++] = pc;
}

/* Create a piece to print a variable from `remd` to `str`
 * Finally, set `remd` to `str`
 */
static void add_expr(docmt_t doc, var_t vr){
	struct piece_s pc;
	pc.is_slice = false;
	// Set pointer to variable
	pc.source.var = vr;
	// Move the remaining pointer forward
	doc->remd = doc->str;
	
	// Create space if necessary
	if(doc->pclen >= doc->pccap){
		doc->pccap <<= 1;
		doc->pieces = realloc(doc->pieces, sizeof(struct piece_s) * doc->pccap);
	}
	doc->pieces[doc->pclen++] = pc;
}



// Create document that stores variables in `nmsp`
docmt_t docmt_new(const char *str, namespace_t nmsp){
	docmt_t doc = malloc(sizeof(struct docmt_s));
	// Create array to hold pieces
	doc->pclen = 0;  doc->pccap = 4;
	doc->pieces = malloc(sizeof(struct piece_s) * doc->pccap);
	// Store namespace to use to store variables
	doc->nmsp = nmsp;

	// Entire string is remaining
	doc->remd = str;
	doc->str = str;
	doc->line_no = 1;
	return doc;
}

// Get associate namespace
namespace_t docmt_get_nmsp(docmt_t doc){ return doc->nmsp; }

// Deallocate document
void docmt_free(docmt_t doc){
	free(doc->pieces);
	free(doc);
}



static int count_lines(const char *start, const char *end){
	int count = 0;
	for(; start < end; start++) if(*start == '\n') count++;
	return count;
}

// Skip blankspace (not including newline)
#define skip_blank(doc) while(isblank(*((doc)->str))) (doc)->str++
// Iterate until end of line is reached
#define skip_line(doc) {\
	while(*((doc)->str) && *((doc)->str) != '\n') (doc)->str++;\
	if(*((doc)->str) == '\n'){ (doc)->str++; (doc)->line_no++; }\
}

// Attempt to parse line of document as expression with optional label and print section
static nmsp_err_t parse_line(docmt_t doc){
	skip_blank(doc);
	if(*(doc->str) == '\0' || *(doc->str) == '#' || *(doc->str) == '\n'){ // Check for comment or end of line
		skip_line(doc);
		return NMSP_ERR_OK;
	}
	
	
	// Parse Labelled Expression
	// --------------------------
	nmsp_err_t err = NMSP_ERR_OK;
	const char *endptr;
	var_t vr = nmsp_define(doc->nmsp, doc->str, &endptr, &err);
	if(err) return err;  // On Parse Error
	doc->line_no += count_lines(doc->str, endptr);
	doc->str = endptr;  // Move pointer past expression on success
	
	
	// Parse Equals
	// -------------
	skip_blank(doc);
	
	// Check for equals sign
	if(*(doc->str) == '='){
		doc->str = ++doc->str;
		add_slice(doc);  // Create slice for content before '='
		
		// Consume print section
		while(*(doc->str) && *(doc->str) != '\n' && *(doc->str) != '#') doc->str++;
		add_expr(doc, vr);  // Create piece to print `vr`
		
	// Check that there is no extra content
	}else if(*(doc->str) != '\n' && *(doc->str) != '#'){
		return PARSE_ERR_EXTRA_CONT;
	}
	
	// Skip Comment
	skip_line(doc);
	
	return NMSP_ERR_OK;
}

static int print_error(docmt_t doc, FILE *stream, nmsp_err_t err){
	if(!stream) return 0;  // Don't print anything to NULL-stream
	
	int count = fprintf(stream, "(Line %i) %s\n", doc->line_no, nmsp_strerror(err));
	
	// Check for Insert Error
	if(err == INSERT_ERR_REDEF){
		char buf[256];
		nmsp_strredef(doc->nmsp, buf, 256);
		count += fprintf(stream, "    Redefinition of \"%s\"\n", buf);
	}else if(err == INSERT_ERR_CIRC){
		char buf[256];
		nmsp_strcirc(doc->nmsp, buf, 256);
		count += fprintf(stream, "    Dependency Chain: %s\n", buf);
	}
	return count;
}

int docmt_parse(docmt_t doc, FILE *errout){
	int err_count = 0;
	while(*(doc->str)){
		// Try to parse line as expression
		nmsp_err_t err = parse_line(doc);
		if(err){  // On Error
			print_error(doc, errout, err);  // Print error
			skip_line(doc);  // Move to next line
			err_count++;
		}
	}
	return err_count;
}


int docmt_fprint(docmt_t doc, FILE *stream, FILE *errout){
	int errcnt = 0;  // Keep track of how many errors occur
	for(size_t i = 0; i < doc->pclen; i++){
		struct piece_s pc = doc->pieces[i];
		if(pc.is_slice){  // Print slice
			if(stream) fprintf(stream, "%.*s",
				pc.source.slice.length,
				pc.source.slice.start
			);
		}else{
			// Evaluate variable
			nmsp_err_t err = nmsp_var_value(NULL, pc.source.var);
			errcnt += !!err;
			
			// Print value or error
			if(stream){
				fputc(' ', stream);  // Buffer result with space
				nmsp_var_fprint(stream, pc.source.var);
				fputc(' ', stream);
			}
			
			// Print any errors
			if(err && errout) print_error(doc, errout, err);
		}	
	}
	
	// Print remaining portion of string
	if(stream) fprintf(stream, "%s", doc->remd);
	
	return errcnt;
}


