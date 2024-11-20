package ca.concordia.broadcast

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.util.Log
import androidx.annotation.RequiresApi
import java.util.Base64

class Receiver : BroadcastReceiver() {
    @RequiresApi(Build.VERSION_CODES.TIRAMISU)
    override fun onReceive(context: Context?, intent: Intent?) {
        val action = intent?.action
        val component = intent?.component?.shortClassName ?: ""
        val data = intent?.dataString ?: ""
        var extras = ""
        intent?.extras?.let {
            for (k in it.keySet()) {
                try {
                    if (extras.isNotEmpty()) {
                        extras += ","
                    }
                    val v = it.getSerializable(k, String::class.java)
                        ?.let { v -> Base64.getEncoder().encodeToString(v.toByteArray()) } ?: ""
                    extras += """["$k","$v"]"""
                } catch (_: Throwable) {

                }
            }
        }
        extras = "[$extras]"
//        try {
//            context?.applicationContext?.unregisterReceiver(Receiver())
//        } catch (_: Throwable) {
//        }
//
//        for (r in intents) {
//            val intentFilter: IntentFilter = IntentFilter(r)
//            try {
//
//
//                    context?.applicationContext?.registerReceiver(Receiver(), intentFilter);
//                } catch (_:Throwable){}
//            }
            val out =
                """"{"action":"$action","component":"$component","data":"$data","extras":$extras}"""
            Log.d("Receiver", "Intent Action: $out")
//        Toast.makeText(context, "Intent Action: $action", Toast.LENGTH_SHORT).show()
        }
    }