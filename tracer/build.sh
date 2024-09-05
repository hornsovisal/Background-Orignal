rm runnable.dex callable.dex
javac -d callable callable/StackTracer_RANDOMID_SAJJAD_POURALI.java
javac -d runnable runnable/StackTracer_RANDOMID_SAJJAD_POURALI.java
d8 --release --output runnable/ runnable/StackTracer_RANDOMID_SAJJAD_POURALI.class
d8 --release --output callable/ callable/StackTracer_RANDOMID_SAJJAD_POURALI.class
mv runnable/classes.dex runnable.dex
mv callable/classes.dex callable.dex