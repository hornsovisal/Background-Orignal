
import android.app.job.JobParameters
import android.app.job.JobService
import android.os.HandlerThread
import android.util.Log
import java.util.logging.Handler

class MyJobService : JobService() {
    override fun onStartJob(params: JobParameters): Boolean {
        val handlerThread = HandlerThread("SomeOtherThread")
        handlerThread.start()

//        val handler: Handler = android.os.Handler(handlerThread.looper)
//        handler.post(Runnable {
//            Log.e(TAG, "Running!!!!!!!!!!!!!")
//            jobFinished(params, true)
//        })

        return true
    }

    override fun onStopJob(params: JobParameters): Boolean {
        Log.d(TAG, "onStopJob() was called")
        return true
    }

    companion object {
        private val TAG: String = MyJobService::class.java.simpleName
    }
}