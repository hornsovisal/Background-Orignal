public class StackTracer_RANDOMID_SAJJAD_POURALI implements Runnable, Comparable<Object> {
    private final Runnable r;

    public StackTracer_RANDOMID_SAJJAD_POURALI(Runnable r) {
        this.r = r;
    }

    @Override
    public void run() {
        if (r != null) {
            r.run();
        }
    }
    @Override
    public int compareTo(Object other) {
        return -1;
    }
}
