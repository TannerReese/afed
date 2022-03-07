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
test/nmsp_test: test/nmsp_test.o nmsp.o bltn.o arith/arith.o util/shunt.o util/mcode.o util/queue.o util/ptree.o
test/nmsp_test.o: test/nmsp_test.c nmsp.h

# Perform nmspession test
nmsp_test: test/nmsp_test
	$<



# Recipes for utilities
arith/arith.o: arith/arith.c arith/arith.h
util/shunt.o: util/shunt.c util/shunt.h util/mcode.h
util/mcode.o: util/mcode.c util/mcode.h
util/ptree.o: util/ptree.c util/ptree.h
util/queue.o: util/queue.c util/queue.h

# Recipes for main library files
nmsp.o: nmsp.c nmsp.h util/vec.h util/queue.h
bltn.o: bltn.c bltn.h arith/arith.h util/ptree.h

# Recipe for primary binary
afed: afed.o docmt.o nmsp.o bltn.o arith/arith.o util/shunt.o util/mcode.o util/queue.o util/ptree.o
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
	@rm -f *.o test/*.o util/*.o arith/*.o
	@echo Removing binaries: $(binaries)
	@rm -f $(binaries)

.PHONY: clean all_test afed_test nmsp_test

