#!/bin/bash

echo
echo "### Testing afed binary"

cd "${0%/*}"  # Move to directory containing script

cases=./cases  # Location of test cases
afed=../afed  # Location of program binary

# Loop over test cases
fails=0
i=0
while [ -f "$cases/c$i.af" ] && [ -f "$cases/c$i.out" ]
do
	echo Checking c$i.af
	# Generate output files and check match
	$afed -n $cases/c$i.af -o $cases/tmp$i.out 2> $cases/tmp$i.err
	
	# Check that output file matches
	if ! diff -s $cases/c$i.out $cases/tmp$i.out ; then
		((fails++))
		echo Errors:
		cat $cases/tmp$i.err
		
	# Check that error file matches if present
	elif [ -f "$cases/c$i.err" ] && ! diff -s $cases/c$i.err $cases/tmp$i.err ; then
		((fails++))
	fi
	
	# Cleanup temporary files
	rm $cases/tmp$i.*
	
	echo Case $i Successful
	echo
	((i++))
done

echo Total Failures: $fails

