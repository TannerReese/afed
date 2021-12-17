CC=gcc
CFLAGS=
binaries=afed test/nmsp_test
libs=m

# Perform all the tests
all_test: nmsp_test afed_test

# Perform afed test
afed_test: test/afed_test.sh afed test/cases/*
	test/afed_test.sh

# Recipe for nmspession tester
test/nmsp_test: test/nmsp_test.o nmsp.o nmsp_dbl.o
test/nmsp_test.o: test/nmsp_test.c nmsp.h

# Perform nmspession test
nmsp_test: test/nmsp_test
	$<



# Recipes for main library files
nmsp.o: nmsp.c nmsp.h
nmsp_dbl.o: nmsp_dbl.c nmsp.h

# Recipe for primary binary
afed: afed.o docmt.o nmsp.o nmsp_dbl.o
afed.o: afed.c docmt.h nmsp.h
docmt.o: docmt.c docmt.h nmsp.h



# Recipe for object files
%.o:
	$(CC) -c $(CFLAGS) -o $@ $(filter %.c,$^)

# Recipe for binaries
$(binaries): %:
	$(CC) $(CFLAGS) -o $@ $(filter %.o,$^) $(addprefix -l,$(libs))

# Remove binary and object files
clean:
	@echo Removing object files
	@rm -f *.o test/*.o
	@echo Removing binaries: $(binaries)
	@rm -f $(binaries)

.PHONY: clean all_test afed_test nmsp_test

