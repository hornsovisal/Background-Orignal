import java.util.concurrent.Callable;

public class StackTracer_RANDOMID_SAJJAD_POURALI<V> implements Callable<V> {
    private final Callable<V> c;

    public StackTracer_RANDOMID_SAJJAD_POURALI(Callable<V> c) {
        this.c = c;
    }

    @Override
    public V call() throws Exception {
        if (c != null) {
            return c.call();
        }
        return null;
    }
}