CC=gcc
CFLAGS=
binaries=afed test/expr_test
libs=m

# Perform all the tests
all_test: expr_test afed_test

# Perform afed test
afed_test: test/afed_test.sh afed test/cases/*
	test/afed_test.sh

# Recipe for expression tester
test/expr_test: test/expr_test.o expr.o expr_dbl.o
test/expr_test.o: test/expr_test.c expr.h expr_dbl.h

# Perform expression test
expr_test: test/expr_test
	$<



# Recipes for main library files
expr.o: expr.c expr.h
expr_dbl.o: expr_dbl.c expr_dbl.h expr.h

# Recipe for primary binary
afed: afed.o docmt.o expr.o expr_dbl.o
afed.o: afed.c docmt.h expr.h expr_dbl.h
docmt.o: docmt.c docmt.h expr.h



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

.PHONY: clean all_test afed_test expr_test

