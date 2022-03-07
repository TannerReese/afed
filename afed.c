#include "nmsp.h"
#include "docmt.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <errno.h>
#include <getopt.h>

#define PROG_NAME "afed"

// Location to read program from, write it to, and log errors
FILE *infile = NULL;
FILE *outfile = NULL;
FILE *errfile = NULL;

bool only_check = 0;
bool allow_overwrite = 1;
bool show_errors = 1;

const char help_msg[] =
	"Usage: " PROG_NAME " [OPTION]... [-i] INFILE [[-o] OUTFILE]\n"
	"\n"
	"Evaluate expressions in place\n"
	"\n"
	"  -i, --input INFILES...  List of files to evaluate\n"
	"  -o, --output OUTFILE    Output file to store result to\n"
	"  -C, --check             Don't output file only check for errors\n"
	"  -n, --no-clobber        Make sure none of the INFILES are used as outputs\n"
	"  -e, --errors ERRFILE    File to send errors to. Sent to stderr if not specified\n"
	"  -E, --no-errors         Don't print any error messages\n"
	"  -h, --help              Print this help message\n"
	"\n"
	"'-' may be used with -o, -i, or -e to indicate STDOUT, STDIN, or STDOUT, respectively\n"
;

const char usage_msg[] = "Usage: " PROG_NAME " [OPTION]... [-i] INFILE [-o OUTFILE]\nUse --help for more information";

struct option longopts[] = {
	{"input", required_argument, NULL, 'i'},
	{"output", required_argument, NULL, 'o'},
	{"check", no_argument, NULL, 'C'},
	{"no-clobber", no_argument, NULL, 'n'},
	{"errors", required_argument, NULL, 'e'},
	{"no-errors", required_argument, NULL, 'E'},
	{"help", no_argument, NULL, 'h'},
	{0}
};

void leave(int code){
	// Close open file descriptors
	if(infile) fclose(infile);
	if(outfile && outfile != infile) fclose(outfile);
	if(errfile) fclose(errfile);
	
	exit(code);
}

#define usage(code, ...) { fprintf(stderr, __VA_ARGS__); puts(usage_msg); leave(code); }

void parse_opt(int key){
	switch(key){
		case -1:  // Non-Option Arguments
			if(!infile){  // Check for already defined infile
		case 'i':  // Input file
				if(infile) usage(2, "Input file already given\n");
				if(optarg[0] == '-' && optarg[1] == '\0') infile = stdin;
				else infile = fopen(optarg, "r+");  // Allow modification in case infile is used as outfile
				
				// Check that it opened
				if(!infile) usage(1, "Input file \"%s\" did not open: ERRNO %i\n", optarg, errno);
			}else{
		case 'o':  // Output file
				if(outfile) usage(2, "Output file already given\n");
				if(optarg[0] == '-' && optarg[1] == '\0') outfile = stdout;
				else outfile = fopen(optarg, "w");
				
				// Check that it opened
				if(!outfile) usage(1, "Output file \"%s\" did not open: ERRNO %i\n", optarg, errno);
			}
		break;
		
		// Only check for errors
		case 'C': only_check = 1;
		break;
		
		// Don't allow overwriting of input files
		case 'n': allow_overwrite = 0;
		break;
		
		case 'e':  // Error file
			if(errfile) usage(4, "Error file already given\n");	
			if(optarg[0] == '-' && optarg[1] == '\0') errfile = stdout;
			else errfile = fopen(optarg, "w");
			
			// Check that it opened
			if(!errfile) usage(2, "Error File \"%s\" did not open: ERRNO %i\n", optarg, errno);
		break;
		
		// Don't show any errors
		case 'E': show_errors = 0;
		break;
		
		// Print help message
		case 'h':
			puts(help_msg);
			leave(0);
	}
}




// Read contents of file into heap allocated string
char *read_file(FILE *fl);

int main(int argc, char *argv[]){
	// Parse Command Line Arguments
	// -----------------------------
	int c;
	while((c = getopt_long(argc, argv, "i:o:e:nEC", longopts, NULL)) != -1) parse_opt(c);
	for(int i = optind; i < argc; i++){
		optarg = argv[i];
		parse_opt(-1);
	}
	
	// Make sure an infile was given
	if(!infile) usage(4, "No Input file given\n");
	
	// Set default outfile as infile unless --no-clobber present
	if(!outfile && !only_check){
		if(allow_overwrite) outfile = infile == stdin ? stdout : infile;
		else usage(3, "No Output file given and --no-clobber present\n");
	}
	
	// Set default error file if not given
	if(!errfile) errfile = stderr;
	
	// Get content of input file and process
	char *prog = read_file(infile);
	
	// Parse and evaluate
	namespace_t nmsp = nmsp_new(true);
	docmt_t doc = docmt_new(prog, nmsp);
	int errcnt = docmt_parse(doc, show_errors ? errfile : NULL);  // Parse file
	fseek(infile, 0, SEEK_SET);  // Move `infile` back to beginning
	
	// Print out to new file
	errcnt += docmt_fprint(doc,
		only_check ? NULL : outfile,
		show_errors ? errfile : NULL
	);
	
	if(only_check){
		// Print out number of errors
		if(errcnt > 0) fprintf(errfile, "%i Parse Error%c\n", errcnt, errcnt > 1 ? 's' : ' ');
		else fprintf(errfile, "No Parse Errors\n");
	}
	
	// Cleanup heap allocations
	free(prog);
	docmt_free(doc);
	nmsp_free(nmsp);
	leave(0);
}



char *read_file(FILE *fl){
	size_t len = 0, cap = 1024;  // Begin with 1024 bytes of capacity
	char *cont = malloc(sizeof(char) * cap);
	while((len += fread(cont + len, 1, cap - len, fl)) >= cap){  // Consume input until EOF
		cap <<= 1;
		cont = realloc(cont, sizeof(char) * cap);
	}
	return cont;
}

