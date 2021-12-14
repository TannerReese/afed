CC=gcc
CFLAGS=
tests=dbl_test
binaries=afed $(addprefix test/,$(tests))
libs=m

# Perform all the tests
test_all: $(tests)

# Recipes for test files
test/dbl_test: test/dbl_test.o expr.o expr_dbl.o
test/dbl_test.o: test/dbl_test.c expr.h expr_dbl.h

# Recipes for main library files
expr.o: expr.c expr.h
expr_dbl.o: expr_dbl.c expr_dbl.h expr.h

# Recipe for primary binary
docmt.o: docmt.c expr.h
afed.o: afed.c docmt.h expr.h expr_dbl.h
afed: afed.o docmt.o expr.o expr_dbl.o



# Recipe for performing a test
$(tests): %: test/%
	test/$@

# Recipe for object files
%.o:
	$(CC) -c $(CFLAGS) -o $@ $(filter %.c,$^)

# Recipe for test binaries
$(binaries): %:
	$(CC) $(CFLAGS) -o $@ $(filter %.o,$^) $(addprefix -l,$(libs))

# Remove binary and object files
clean:
	@echo Removing object files
	@rm -f *.o test/*.o
	@echo Removing binaries: $(binaries)
	@rm -f $(binaries)

.PHONY: clean test_all $(tests)

