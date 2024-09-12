@file:Suppress("UNUSED", "UNUSED_PARAMETER", "SpellCheckingInspection")

package ca.concordia.madiba.cryptologger

import android.annotation.SuppressLint
import android.app.ActivityManager
import android.app.AlarmManager
import android.app.Notification
import android.app.PendingIntent
import android.app.Service
import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.location.Location
import android.os.Handler
import android.os.Looper
import android.os.Message
import android.os.StrictMode
import android.os.StrictMode.ThreadPolicy
import android.os.WorkSource
import android.provider.Settings.Secure
import dalvik.system.BaseDexClassLoader
import dalvik.system.InMemoryDexClassLoader
import io.github.libxposed.api.XposedInterface
import io.github.libxposed.api.XposedInterface.AfterHookCallback
import io.github.libxposed.api.XposedInterface.BeforeHookCallback
import io.github.libxposed.api.XposedModule
import io.github.libxposed.api.XposedModuleInterface.ModuleLoadedParam
import io.github.libxposed.api.XposedModuleInterface.PackageLoadedParam
import io.github.libxposed.api.annotations.AfterInvocation
import io.github.libxposed.api.annotations.BeforeInvocation
import io.github.libxposed.api.annotations.XposedHooker
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import java.io.ByteArrayOutputStream
import java.io.File
import java.lang.reflect.Method
import java.lang.reflect.Modifier
import java.net.Proxy
import java.net.Socket
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.security.AlgorithmParameters
import java.security.Key
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.Certificate
import java.security.spec.AlgorithmParameterSpec
import java.util.Base64
import java.util.Enumeration
import java.util.UUID
import java.util.concurrent.AbstractExecutorService
import java.util.concurrent.Callable
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Executor
import java.util.concurrent.ForkJoinPool
import java.util.concurrent.FutureTask
import java.util.concurrent.ScheduledThreadPoolExecutor
import java.util.concurrent.ThreadPoolExecutor
import java.util.zip.Adler32
import java.util.zip.GZIPOutputStream
import javax.crypto.Cipher
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocketFactory
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager
import kotlin.random.Random

private lateinit var module: ModuleMain
private lateinit var packageName: String
private lateinit var globalClassLoader: ClassLoader

class ModuleMain(base: XposedInterface, param: ModuleLoadedParam) : XposedModule(base, param) {
    private val addedDexFiles: MutableList<String> = mutableListOf()
    private val latitude: Double = 45.4956206
    private val longitude: Double = -73.580828
    private val randmap: MutableMap<String, String> = mutableMapOf()
    private val registeredReceivers: MutableSet<String> = mutableSetOf()

    //    private val endpoint = "https://172.30.143.86:8888/endpoint"
    private lateinit var webSocket: WebSocket
    private val webSocketUrl = "wss://172.30.143.74:8888/endpoint"
//    private val webSocketUrl = "wss://172.30.143.74:8888/endpoint"
//    private val webSocketUrl = "wss://192.168.70.100:8888/endpoint"
    private var isDeviceIdSent = false
    //val url = URL("https://192.168.70.100:8888/endpoint")

    init {
        log("ModuleMain at " + param.processName)
        module = this
        initWebSocket()
        packageName = param.processName
    }

    private fun initWebSocket() {
        val trustAllCerts = arrayOf<TrustManager>(@SuppressLint("CustomX509TrustManager")
        object : X509TrustManager {
            @SuppressLint("TrustAllX509TrustManager")
            override fun checkClientTrusted(
                chain: Array<java.security.cert.X509Certificate>,
                authType: String
            ) {
            }

            @SuppressLint("TrustAllX509TrustManager")
            override fun checkServerTrusted(
                chain: Array<java.security.cert.X509Certificate>,
                authType: String
            ) {
            }

            override fun getAcceptedIssuers(): Array<java.security.cert.X509Certificate> = arrayOf()
        })

        val sslContext: SSLContext = SSLContext.getInstance("TLS")
        sslContext.init(null, trustAllCerts, SecureRandom())
        val sslSocketFactory: SSLSocketFactory = sslContext.socketFactory

        val client = OkHttpClient.Builder()
            .sslSocketFactory(sslSocketFactory, trustAllCerts[0] as X509TrustManager)
            .hostnameVerifier { _, _ -> true }
            .proxy(Proxy.NO_PROXY)
//            .socketFactory(NoProxySocketFactory())
            .build()
//        val client = OkHttpClient
        connectWebSocket(client)
    }

    private fun connectWebSocket(client: OkHttpClient) {
        val request = Request.Builder().url(webSocketUrl).build()
        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                log("WebSocket connection opened")
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                log("Received message: $text")
            }

            override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                log("Received bytes: ${bytes.hex()}")
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                log("WebSocket closing: $code / $reason")
                webSocket.close(1000, null)
                log("Attempting to reconnect WebSocket...")
                Handler(Looper.getMainLooper()).postDelayed({
                    connectWebSocket(client)
                }, 1000)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                log("WebSocket error: $packageName -> ${t.message}")
                log("Attempting to reconnect WebSocket...")
                Handler(Looper.getMainLooper()).postDelayed({
                    connectWebSocket(client)
                }, 1000)
//                connectWebSocket(client)
            }
        })
    }

    fun sendDataToServer(
        objName: String,
        className: String,
        methodName: String,
        data: String,
        hashcode: Int,
        stackTrace: String
    ) {
        val a = ActivityManager.RunningAppProcessInfo()
        ActivityManager.getMyMemoryState(a)

        val policy = ThreadPolicy.Builder().permitAll().build()
        StrictMode.setThreadPolicy(policy)

        val completedData = if (data.isNotEmpty()) {
            """{"$objName":{"class_name":"$className","method_name":"$methodName", "hashcode": $hashcode, $data,"pid":${android.os.Process.myPid()},"tid":${android.os.Process.myTid()},"jtid":${Thread.currentThread().id},"uid":${android.os.Process.myUid()},"package_name":"$packageName","ts":${System.currentTimeMillis()}, "importance":${a.importance}, "stack_trace": "$stackTrace"}}"""
        } else {
            """{"$objName":{"class_name":"$className","method_name":"$methodName", "hashcode": $hashcode, "pid":${android.os.Process.myPid()},"tid":${android.os.Process.myTid()},"jtid":${Thread.currentThread().id},"uid":${android.os.Process.myUid()},"package_name":"$packageName","ts":${System.currentTimeMillis()}, "importance":${a.importance}, "stack_trace": "$stackTrace"}}"""
        }
//        module.log(completedData)
//        return
        if (::webSocket.isInitialized) {
            webSocket.send(completedData)
        } else {
            log("WebSocket is not initialized")
        }
    }


    //start of crypto
    @XposedHooker
    class DoFinalHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalHook {
                return DoFinalHook(Random.nextInt())
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DoFinalHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinal",
                    callback.thisObject!!.javaClass.name,
                    "do_final",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class DoFinalByteHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalByteHook {
                return DoFinalByteHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DoFinalByteHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinalByte",
                    callback.thisObject!!.javaClass.name,
                    "do_final_byte",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class DoFinalByteIntHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var output: Any
        private lateinit var outputOffset: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalByteIntHook {

                return DoFinalByteIntHook(Random.nextInt()).apply {
                    this.output = callback.args[0]
                    this.outputOffset = callback.args[1]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DoFinalByteIntHook) {
                val out = """"arg_output": "${
                    Base64.getEncoder().encodeToString(context.output as ByteArray)
                }", "arg_output_offset": ${context.outputOffset as Int},"ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinalByteInt",
                    callback.thisObject!!.javaClass.name,
                    "do_final_byte_int",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class DoFinalByteIntIntHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalByteIntIntHook {
                return DoFinalByteIntIntHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DoFinalByteIntIntHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int},"ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinalByteIntInt",
                    callback.thisObject!!.javaClass.name,
                    "do_final_byte_int_int",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class DoFinalByteIntIntByteHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any
        private lateinit var output: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalByteIntIntByteHook {
                return DoFinalByteIntIntByteHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                    this.output = callback.args[3]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DoFinalByteIntIntByteHook) {
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int}, "arg_output": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }","ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinalByteIntIntByte",
                    callback.thisObject!!.javaClass.name,
                    "do_final_byte_int_int_byte",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class DoFinalByteIntIntByteIntHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any
        private lateinit var output: Any
        private lateinit var outputOffset: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalByteIntIntByteIntHook {
                return DoFinalByteIntIntByteIntHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                    this.output = callback.args[3]
                    this.outputOffset = callback.args[4]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: DoFinalByteIntIntByteIntHook
            ) {
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int}, "arg_output": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_output_offset": ${context.outputOffset as Int},"ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinalByteIntIntByteInt",
                    callback.thisObject!!.javaClass.name,
                    "do_final_byte_int_int_byte_int",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class DoFinalByteBufferByteBufferHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var output: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoFinalByteBufferByteBufferHook {
                return DoFinalByteBufferByteBufferHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.output = callback.args[1]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: DoFinalByteBufferByteBufferHook
            ) {
                val input = (context.input as ByteBuffer).array()
                val output = (context.output as ByteBuffer).array()
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(input)
                }", "arg_output": "${
                    Base64.getEncoder().encodeToString(output)
                }","ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "DoFinalByteBufferByteBuffer",
                    callback.thisObject!!.javaClass.name,
                    "do_final_byte_buffer_byte_buffer",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntCertificateHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var certificate: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntCertificateHook {
                return InitIntCertificateHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.certificate = callback.args[1]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: InitIntCertificateHook) {
//                "iv":"${Base64.getEncoder().encodeToString((callback.thisObject as Cipher).iv)}",
                val out = """"arg_opcode": ${context.opcode as Int}, "arg_certificate": "${
                    Base64.getEncoder().encodeToString((context.certificate as Certificate).encoded)
                }", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntCertificate",
                    callback.thisObject!!.javaClass.name,
                    "init_int_certificate",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntCertificateSecureRandomHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var certificate: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntCertificateSecureRandomHook {
                return InitIntCertificateSecureRandomHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.certificate = callback.args[1]
//                    this.random = callback.args[2]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: InitIntCertificateSecureRandomHook
            ) {
                val out = """"arg_opcode": ${context.opcode as Int}, "arg_certificate": "${
                    Base64.getEncoder().encodeToString((context.certificate as Certificate).encoded)
                }", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntCertificate",
                    callback.thisObject!!.javaClass.name,
                    "init_int_certificate_secure_random",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntKeyHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var key: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntKeyHook {
                return InitIntKeyHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.key = callback.args[1]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: InitIntKeyHook) {
                val key = when (val result = (context.key as Key).encoded) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"arg_opcode": ${context.opcode as Int}, "arg_key": "$key", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntKey",
                    callback.thisObject!!.javaClass.name,
                    "init_int_key",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntKeyAlgorithmParametersHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var key: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntKeyAlgorithmParametersHook {
                return InitIntKeyAlgorithmParametersHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.key = callback.args[1]
//                    this.params = callback.args[2]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: InitIntKeyAlgorithmParametersHook
            ) {
                val key = when (val result = (context.key as Key).encoded) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"arg_opcode": ${context.opcode as Int}, "arg_key": "$key", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntKey",
                    callback.thisObject!!.javaClass.name,
                    "init_int_key_algorithm_parameters",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntKeyAlgorithmParameterSpecHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var key: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntKeyAlgorithmParameterSpecHook {
                return InitIntKeyAlgorithmParameterSpecHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.key = callback.args[1]
//                    this.params = callback.args[2]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: InitIntKeyAlgorithmParameterSpecHook
            ) {
                val key = when (val result = (context.key as Key).encoded) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"arg_opcode": ${context.opcode as Int}, "arg_key": "$key", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntKey",
                    callback.thisObject!!.javaClass.name,
                    "init_int_key_algorithm_parameter_spec",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntKeyAlgorithmParameterSpecSecureRandomHook(private val magic: Int) :
        XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var key: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntKeyAlgorithmParameterSpecSecureRandomHook {
                return InitIntKeyAlgorithmParameterSpecSecureRandomHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.key = callback.args[1]
//                    this.params = callback.args[2]
//                    this.random = callback.args[3]

                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: InitIntKeyAlgorithmParameterSpecSecureRandomHook
            ) {
                val key = when (val result = (context.key as Key).encoded) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"arg_opcode": ${context.opcode as Int}, "arg_key": "$key", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntKey",
                    callback.thisObject!!.javaClass.name,
                    "init_int_key_algorithm_parameter_spec_secure_random",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntKeyAlgorithmParametersSecureRandomHook(private val magic: Int) :
        XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var key: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntKeyAlgorithmParametersSecureRandomHook {
                return InitIntKeyAlgorithmParametersSecureRandomHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.key = callback.args[1]
//                    this.params = callback.args[2]
//                    this.random = callback.args[3]

                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: InitIntKeyAlgorithmParametersSecureRandomHook
            ) {
                val key = when (val result = (context.key as Key).encoded) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"arg_opcode": ${context.opcode as Int}, "arg_key": "$key", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntKey",
                    callback.thisObject!!.javaClass.name,
                    "init_int_key_algorithm_parameters_secure_random",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class InitIntKeySecureRandomHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var opcode: Any
        private lateinit var key: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): InitIntKeySecureRandomHook {
                return InitIntKeySecureRandomHook(Random.nextInt()).apply {
                    this.opcode = callback.args[0]
                    this.key = callback.args[1]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: InitIntKeySecureRandomHook) {
                val key = when (val result = (context.key as Key).encoded) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out =
                    """"arg_opcode": ${context.opcode as Int}, "arg_key": "$key", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "InitIntKey",
                    callback.thisObject!!.javaClass.name,
                    "init_int_key_secure_random",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateByteHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateByteHook {

                return UpdateByteHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateByteHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateByte",
                    callback.thisObject!!.javaClass.name,
                    "update_byte",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateByteIntIntHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateByteIntIntHook {
                return UpdateByteIntIntHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateByteIntIntHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int},"ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateByteIntInt",
                    callback.thisObject!!.javaClass.name,
                    "update_byte_int_int",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateByteIntIntByteHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any
        private lateinit var output: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateByteIntIntByteHook {
                return UpdateByteIntIntByteHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                    this.output = callback.args[3]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateByteIntIntByteHook) {
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int}, "arg_output": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }","ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateByteIntIntByte",
                    callback.thisObject!!.javaClass.name,
                    "update_byte_int_int_byte",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateByteIntIntByteIntHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any
        private lateinit var output: Any
        private lateinit var outputOffset: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateByteIntIntByteIntHook {
                return UpdateByteIntIntByteIntHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                    this.output = callback.args[3]
                    this.outputOffset = callback.args[4]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateByteIntIntByteIntHook) {
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int}, "arg_output": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_output_offset": ${context.outputOffset as Int},"ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateByteIntIntByteInt",
                    callback.thisObject!!.javaClass.name,
                    "update_byte_int_int_byte_int",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateByteBufferByteBufferHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var output: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateByteBufferByteBufferHook {
                return UpdateByteBufferByteBufferHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.output = callback.args[1]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(
                callback: AfterHookCallback,
                context: UpdateByteBufferByteBufferHook
            ) {
                val input = (context.input as ByteBuffer).array()
                val output = (context.output as ByteBuffer).array()
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(input)
                }", "arg_output": "${
                    Base64.getEncoder().encodeToString(output)
                }","ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateByteBufferByteBuffer",
                    callback.thisObject!!.javaClass.name,
                    "update_byte_buffer_byte_buffer",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateAADByteHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateAADByteHook {

                return UpdateAADByteHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateAADByteHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateAADByte",
                    callback.thisObject!!.javaClass.name,
                    "update_aad_byte",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateAADByteIntIntHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var input: Any
        private lateinit var inputOffset: Any
        private lateinit var inputLen: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateAADByteIntIntHook {
                return UpdateAADByteIntIntHook(Random.nextInt()).apply {
                    this.input = callback.args[0]
                    this.inputOffset = callback.args[1]
                    this.inputLen = callback.args[2]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateAADByteIntIntHook) {
                val ret = when (val result = callback.result) {
                    is ByteArray -> Base64.getEncoder().encodeToString(result)
                    else -> ""
                }
                val out = """"arg_input": "${
                    Base64.getEncoder().encodeToString(context.input as ByteArray)
                }", "arg_input_offset": ${context.inputOffset as Int}, "arg_input_len": ${context.inputLen as Int},"ret":"$ret", "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateAADByteIntInt",
                    callback.thisObject!!.javaClass.name,
                    "update_aad_byte_int_int",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class UpdateAADByteBufferHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var src: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): UpdateAADByteBufferHook {
                return UpdateAADByteBufferHook(Random.nextInt()).apply {
                    this.src = callback.args[0]
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: UpdateAADByteBufferHook) {
                val src = (context.src as ByteBuffer).array()
                val out = """"arg_src": "${
                    Base64.getEncoder().encodeToString(src)
                }","ret":${callback.result as Int}, "algorithm": "${(callback.thisObject as Cipher).algorithm}""""
                module.sendDataToServer(
                    "UpdateAADByteBuffer",
                    callback.thisObject!!.javaClass.name,
                    "update_aad_byte_buffer",
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    // //start of GPS
    @XposedHooker
    class LatitudeHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): LatitudeHook {
                return LatitudeHook(Random.nextInt())
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: LatitudeHook) {
                callback.result = module.latitude
            }
        }
    }

    @XposedHooker
    class LongitudeHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): LongitudeHook {
                return LongitudeHook(Random.nextInt())
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: LongitudeHook) {
                callback.result = module.longitude
            }
        }
    }

    //start of Dex Loader
    @XposedHooker
    class DexLoaderHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var path: String

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DexLoaderHook {
                return DexLoaderHook(Random.nextInt())
                    .apply {
                        this.path = callback.args[0] as String
                    }

            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DexLoaderHook) {
                module.scanDexFile(context.path, globalClassLoader)
            }
        }
    }


    @XposedHooker
    class StartForegroundHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): StartForegroundHook {
                val firstArg = callback.args[0] as Int
                val type = try {
                    callback.args[2] as Int
                } catch (_: Throwable) {
                    0
                }
                val out = """"id":$firstArg,"type":$type"""
                module.sendDataToServer(
                    "StartForeground",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return StartForegroundHook(Random.nextInt())
            }
        }
    }

    @XposedHooker
    class OnStartCommandHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var out: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): OnStartCommandHook {
//                module.log("called service implementation: ${this::class.java.name}")
                val intent = try {
                    callback.args[0] as Intent
                } catch (_: Throwable) {
                    null
                }
                val action = intent?.action ?: ""
                val component = intent?.component?.shortClassName ?: ""
                val data = intent?.dataString ?: ""

                val flags = callback.args[1] as Int
                val startId = callback.args[2] as Int
                var extras = ""
                intent?.extras?.let {
                    for (k in it.keySet()) {
                        try {
                            if (extras.isNotEmpty()) {
                                extras += ","
                            }
                            val v = it.getSerializable(k,String::class.java)?.let{ v -> Base64.getEncoder().encodeToString(v.toByteArray())}?:""
                            extras += """["$k","$v"]"""
                        } catch (_: Throwable) {

                        }
                    }
                }
                extras = "[$extras]"

                val out =
                    """"action":"$action","component":"$component","data":"$data","extras":$extras,"flags":$flags,"start_id":$startId"""

                Thread {
                    try {
                        module.sendDataToServer(
                            "OnStartCommand",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            out,
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                        OnStartCommandHook(Random.nextInt())
                    } catch (_: Throwable) {

                    }

                }.start()

                return OnStartCommandHook(Random.nextInt()).apply {
                    this.out = out
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: OnStartCommandHook) {
                module.sendDataToServer(
                    "OnStartCommand",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    context.out as String,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class OnBindHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var out: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): OnBindHook {
                val intent = callback.args[0] as Intent
                val action = intent.action ?: ""
                val component = intent.component?.shortClassName ?: ""
                val data = intent.dataString ?: ""
                var extras = ""
                intent.extras?.let {
                    for (k in it.keySet()) {
                        try {
                            if (extras.isNotEmpty()) {
                                extras += ","
                            }
                            val v = it.getSerializable(k,String::class.java)?.let{ v -> Base64.getEncoder().encodeToString(v.toByteArray())}?:""
                            extras += """["$k","$v"]"""
                        } catch (_: Throwable) {

                        }
                    }
                }
                extras = "[$extras]"

                val out =
                    """"action":"$action","component":"$component","data":"$data","extras":$extras"""



                Thread {
                    try {
                        module.sendDataToServer(
                            "OnBind",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            out,
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                        OnBindHook(Random.nextInt())
                    } catch (_: Throwable) {

                    }

                }.start()

                return OnBindHook(Random.nextInt()).apply {
                    this.out = out
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: OnBindHook) {
                module.sendDataToServer(
                    "OnBind",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    context.out as String,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class SetImplHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): SetImplHook {
                val type = callback.args[0] as Int
                val triggerAtMillis = callback.args[1] as Long
                val windowMillis = callback.args[2] as Long
                val intervalMillis = callback.args[3] as Long
                val flags = callback.args[4] as Int
                val operationHashcode = (callback.args[5] as PendingIntent?).hashCode()
                val listenerClassName =
                    (callback.args[6] as AlarmManager.OnAlarmListener?)?.javaClass?.name ?: ""
                val listenerTag = (callback.args[7] as String?) ?: ""
                //val targetExecutor = callback.args[8] as Executor?
                //val workSource = callback.args[9] as WorkSource?
                val alarmClock = callback.args[10] as AlarmManager.AlarmClockInfo?
                val alarmClockIntentHashCode = alarmClock?.showIntent?.hashCode() ?: 0
                val alarmClockGetTriggerTime = alarmClock?.triggerTime ?: 0

                val out =
                    """"type":$type, "trigger_at_millis":$triggerAtMillis, "window_millis":$windowMillis,"interval_millis":$intervalMillis, "flags":$flags, "operation_hashcode":$operationHashcode, "listener_class_name":"$listenerClassName", "listener_tag":"$listenerTag", "alarm_clock_intent_hash_code":$alarmClockIntentHashCode, "alarm_clock_get_trigger_time":$alarmClockGetTriggerTime"""
                module.sendDataToServer(
                    "SetImpl",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
                callback.args[1] = System.currentTimeMillis() + 2000
                return SetImplHook(Random.nextInt())
            }
        }
    }


    @XposedHooker
    class SetAlarmClockHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): SetAlarmClockHook {
                val operation = callback.args[1] as PendingIntent
                var creator = operation.toString()
                if (creator == "null") {
                    creator = ""
                }
                val out = """"creator":"$creator""""
                module.sendDataToServer(
                    "SetAlarmClock",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return SetAlarmClockHook(Random.nextInt())
            }
        }
    }

    @XposedHooker
    class PendingIntentGetHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var action: Any
        private lateinit var component: Any
        private lateinit var data: Any
        private lateinit var requestCode: Any
        private lateinit var flags: Any
        private lateinit var context: Any
        private lateinit var intent: Any


        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): PendingIntentGetHook {
//                val intent = callback.args[2] as Intent
                return PendingIntentGetHook(Random.nextInt()).apply {
                    this.requestCode = callback.args[1] as Int
                    this.flags = callback.args[3] as Int
                    this.context = callback.args[0] as Context
                    this.intent = callback.args[2] as Intent

                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: PendingIntentGetHook) {
                val intent = context.intent as Intent
                val action = intent.action ?: ""
                val component = intent.component?.shortClassName ?: ""
                val data = intent.dataString ?: ""
                var extras = ""
                intent.extras?.let {
                    for (k in it.keySet()) {
                        try {
                            if (extras.isNotEmpty()) {
                                extras += ","
                            }
                            val v = it.getSerializable(k,String::class.java)?.let{ v -> Base64.getEncoder().encodeToString(v.toByteArray())}?:""
                            extras += """["$k","$v"]"""
                        } catch (_: Throwable) {

                        }
                    }
                }
                extras = "[$extras]"

                val out =
                    """"request_code":${context.requestCode},"action":"$action","component":"$component","data":"$data","extras":$extras,"flags":${context.flags}""" //
                module.sendDataToServer(
                    "PendingIntentGet",
                    "android.app.PendingIntent",
                    callback.member.name,
                    out,
                    callback.result.hashCode(),
                    b64st()
                )
                try {
                    (context.context as Context).sendBroadcast(intent, "")
                } catch (_: Throwable) {
                }

            }
        }
    }

    @XposedHooker
    class ScheduleHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): ScheduleHook {
                val jobInfo = callback.args[0] as JobInfo
                val jobId = jobInfo.id
                val backoffPolicy = jobInfo.backoffPolicy
                val clipData = jobInfo.clipData?.toString() ?: ""
                val clipGrantFlags = jobInfo.clipGrantFlags
                val estimatedNetworkDownloadBytes = jobInfo.estimatedNetworkDownloadBytes
                val estimatedNetworkUploadBytes = jobInfo.estimatedNetworkUploadBytes
                val flexMillis = jobInfo.flexMillis
                val initialBackoffMillis = jobInfo.initialBackoffMillis
                val intervalMillis = jobInfo.intervalMillis
                val maxExecutionDelayMillis = jobInfo.maxExecutionDelayMillis
                val minLatencyMillis = jobInfo.minLatencyMillis
                val minimumNetworkChunkBytes = jobInfo.minimumNetworkChunkBytes

                @Suppress("DEPRECATION")
                val networkType = jobInfo.networkType
                val priority = jobInfo.priority
                val triggerContentMaxDelay = jobInfo.triggerContentMaxDelay
                val triggerContentUpdateDelay = jobInfo.triggerContentUpdateDelay
                val triggerContentUris = jobInfo.triggerContentUris?.joinToString(",") ?: ""
                val isExpedited = jobInfo.isExpedited
                val isImportantWhileForeground = jobInfo.isImportantWhileForeground
                val isPeriodic = jobInfo.isPeriodic
                val isPersisted = jobInfo.isPersisted
                val isPrefetch = jobInfo.isPrefetch
                val isRequireBatteryNotLow = jobInfo.isRequireBatteryNotLow
                val isRequireCharging = jobInfo.isRequireCharging
                val isRequireDeviceIdle = jobInfo.isRequireDeviceIdle
                val isRequireStorageNotLow = jobInfo.isRequireStorageNotLow
                val service = jobInfo.service.flattenToString()
                val requiredNetworkCapabilities =
                    jobInfo.requiredNetwork?.capabilities?.joinToString(",") ?: ""
                val requiredNetworkTransportTypes =
                    jobInfo.requiredNetwork?.transportTypes?.joinToString(",") ?: ""
                val out =
                    """"job_id": $jobId,"backoff_policy": $backoffPolicy,"clip_data": "$clipData","clip_grant_flags": $clipGrantFlags,"estimated_network_download_bytes": $estimatedNetworkDownloadBytes,"estimated_network_upload_bytes": $estimatedNetworkUploadBytes,"flex_millis": $flexMillis,"initial_backoff_millis": $initialBackoffMillis,"interval_millis": $intervalMillis,"max_execution_delay_millis": $maxExecutionDelayMillis,"min_latency_millis": $minLatencyMillis,"minimum_network_chunk_bytes": $minimumNetworkChunkBytes,"network_type": $networkType,"priority": $priority,"service": "$service","required_network_capabilities": [$requiredNetworkCapabilities],"required_network_transport_types": [$requiredNetworkTransportTypes],"trigger_content_max_delay": $triggerContentMaxDelay,"trigger_content_update_delay": $triggerContentUpdateDelay,"trigger_content_uris": "$triggerContentUris","is_expedited": $isExpedited,"is_important_while_foreground": $isImportantWhileForeground,"is_periodic": $isPeriodic,"is_persisted": $isPersisted,"is_prefetch": $isPrefetch,"is_require_battery_not_low": $isRequireBatteryNotLow,"is_require_charging": $isRequireCharging,"is_require_device_idle": $isRequireDeviceIdle,"is_require_storage_not_low": $isRequireStorageNotLow""".trimIndent()
                module.sendDataToServer(
                    "JobScheduler",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return ScheduleHook(Random.nextInt())
            }
        }
    }

    @XposedHooker
    class OnStartJobHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var out: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): OnStartJobHook {
                val jobParameters = callback.args[0] as JobParameters
                val jobId = jobParameters.jobId

                val out = """"job_id":$jobId,"start_ts":${System.currentTimeMillis()}"""


                Thread {
                    try {
                        module.sendDataToServer(
                            "OnStartJob",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            out,
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                        OnStartJobHook(Random.nextInt())
                    } catch (_: Throwable) {

                    }

                }.start()

                return OnStartJobHook(Random.nextInt()).apply {
                    this.out = out
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: OnStartJobHook) {
                module.sendDataToServer(
                    "OnStartJob",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    context.out as String,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class ScheduleInternalHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): ScheduleInternalHook {
                val firstArg = callback.args[0]
                val idField = firstArg.javaClass.getDeclaredField("id")
                idField.isAccessible = true
                val workId = idField.get(firstArg)
                val jobId = callback.args[1] as Int
                val out = """"job_id":$jobId,"work_id":"$workId""""
                module.sendDataToServer(
                    "ScheduleInternal",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
                return ScheduleInternalHook(Random.nextInt())
            }
        }
    }

    @XposedHooker
    class DoWorkHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var out: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): DoWorkHook {
                val workId: Any? = try {
                    var currentClass: Class<*>? = callback.thisObject?.javaClass
                    var id: Any? = null
                    while (currentClass != null && id == null) {
                        try {
                            val getIdMethod = currentClass.getMethod("getId")
                            getIdMethod.isAccessible = true
                            id = getIdMethod.invoke(callback.thisObject)
                        } catch (e: NoSuchMethodException) {
//                            module.log("No such method in class ${currentClass.name}, moving to superclass")
                            currentClass = currentClass.superclass
                        }
                    }
                    id
                } catch (_: Throwable) {
//                    module.log("Failed to get work ID: ${e.message}")
                    null
                }


                val out = """"work_id":"$workId""""

                Thread {
                    try {
                        module.sendDataToServer(
                            "DoWork",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            out,
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                        DoWorkHook(Random.nextInt())
                    } catch (_: Throwable) {

                    }

                }.start()

                return DoWorkHook(Random.nextInt()).apply {
                    this.out = out
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: DoWorkHook) {
                module.sendDataToServer(
                    "DoWork",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    context.out as String,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class WorkRequestHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): WorkRequestHook {
                val workId = callback.args[0] as UUID
                val out = """"work_id":"$workId""""
                module.sendDataToServer(
                    "RequestWork",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return WorkRequestHook(Random.nextInt())
            }
        }
    }

    @XposedHooker
    class OnReceiveHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var out: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): OnReceiveHook {
                val intent = callback.args[1] as Intent
                val action = intent.action ?: ""
                val component = intent.component?.shortClassName ?: ""
                val data = intent.dataString ?: ""
                var extras = ""
                intent.extras?.let {
                    for (k in it.keySet()) {
                        try {
                            if (extras.isNotEmpty()) {
                                extras += ","
                            }
                            val v = it.getSerializable(k,String::class.java)?.let{ v -> Base64.getEncoder().encodeToString(v.toByteArray())}?:""
                            extras += """["$k","$v"]"""
                        } catch (_: Throwable) {

                        }
                    }
                }
                extras = "[$extras]"

                val out =
                    """"action":"$action","component":"$component","data":"$data","extras":$extras,"start_ts":${System.currentTimeMillis()}"""
                Thread {
                    try {
                        module.sendDataToServer(
                            "OnReceiveHook",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            out,
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                        OnReceiveHook(Random.nextInt())
                    } catch (_: Throwable) {

                    }

                }.start()

                return OnReceiveHook(Random.nextInt()).apply {
                    this.out = out
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: OnReceiveHook) {

                module.sendDataToServer(
                    "OnReceiveHook",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    context.out as String,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }

    @XposedHooker
    class FirstArgIntentHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): FirstArgIntentHook {
                val intent = callback.args[0] as Intent
                val action = intent.action ?: ""
                val component = intent.component?.shortClassName ?: ""
                val data = intent.dataString ?: ""
                var extras = ""
                intent.extras?.let {
                    for (k in it.keySet()) {
                        try {
                            if (extras.isNotEmpty()) {
                                extras += ","
                            }
                            val v = it.getSerializable(k,String::class.java)?.let{ v -> Base64.getEncoder().encodeToString(v.toByteArray())}?:""
                            extras += """["$k","$v"]"""
                        } catch (_: Throwable) {

                        }
                    }
                }
                extras = "[$extras]"

                val out =
                    """"action":"$action","component":"$component","data":"$data","extras":$extras"""
                module.sendDataToServer(
                    "Intent",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return FirstArgIntentHook(Random.nextInt())
            }
        }
    }


    @XposedHooker
    class BindServiceHook(private val magic: Int) : XposedInterface.Hooker {
        private lateinit var intent: Any

        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): BindServiceHook {
                val intent = callback.args[0] as Intent
                val action = intent.action ?: ""
                val component = intent.component?.shortClassName ?: ""
                val data = intent.dataString ?: ""
//                if (data == "null") {
//                    data = ""
//                }
                var extras = ""
                intent.extras?.let {
                    for (k in it.keySet()) {
                        try {
                            if (extras.isNotEmpty()) {
                                extras += ","
                            }
                            val v = it.getSerializable(k,String::class.java)?.let{ v -> Base64.getEncoder().encodeToString(v.toByteArray())}?:""
                            extras += """["$k","$v"]"""
                        } catch (_: Throwable) {

                        }
                    }
                }
                extras = "[$extras]"

                val out =
                    """"action":"$action","component":"$component","data":"$data","extras":$extras"""
                module.sendDataToServer(
                    "Intent",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return BindServiceHook(Random.nextInt()).apply {
                    this.intent = intent
                }
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: BindServiceHook) {
                val intent = context.intent as Intent
                try {
                    (callback.thisObject as Context).sendBroadcast(intent, "")
                } catch (_: Throwable) {
                }

            }
        }
    }

    @XposedHooker
    class HandlerPostHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): HandlerPostHook {
                if (callback.args.size == 2) {
                    try {
                        if (callback.args[1] as Long > 1000)
                            callback.args[1] = 1000
                    } catch (_: Throwable) {
                        if (callback.args[1] as Int > 1000)
                            callback.args[1] = 1000
                    }
                } else if (callback.args.size == 3) {
                    try {
                        if (callback.args[2] as Long > 1000)
                            callback.args[2] = 1000
                    } catch (_: Throwable) {
                        if (callback.args[2] as Int > 1000)
                            callback.args[2] = 1000
                    }
                }

                var n = "${android.os.Process.myPid()}_${android.os.Process.myTid()}".padStart(
                    23,
                    '_'
                )
                canary()?.let {
                    if (it[1] == '-') {
                        n = n.drop(8)
                        n = it[0] + getRandomString(7) + n
                        module.sendDataToServer(
                            "Canary",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            """"canary":"$n","target":"${it.substringAfter("-")}"""",
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                    } else if (it.length == 8) {
//                        module.log("@@@@@@@@$it")
                        n = n.drop(8)
                        n = it + n
                    }
//                    else{
//                        module.log("@@@@@@@@X$it")
//                    }
                }
                val targetClassName = "StackTracer_$n"
                val wrapperRunnableClass = try {
                    globalClassLoader.loadClass(targetClassName)!!
                } catch (_: Throwable) {
                    val classBytes = createRunnableTracerFunction(n)
                    val classLoader = InMemoryDexClassLoader(
                        ByteBuffer.wrap(classBytes),
                        globalClassLoader
                    )
                    classLoader.loadClass(targetClassName)
                }
                val oldRunnable = callback.args[0] as Runnable
                val runnableInstance = wrapperRunnableClass.getConstructor(Runnable::class.java)
                    .newInstance(Runnable {
                        oldRunnable.run()
                    }) as Runnable
                callback.args[0] = runnableInstance


                return HandlerPostHook(Random.nextInt())
            }

        }
    }


    @XposedHooker
    class TraceThreadHookFirst(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): TraceThreadHookFirst {
                var n = "${android.os.Process.myPid()}_${android.os.Process.myTid()}".padStart(
                    23,
                    '_'
                )
                canary()?.let {
                    if (it[1] == '-') {
                        n = n.drop(8)
                        n = it[0] + getRandomString(7) + n
                        module.sendDataToServer(
                            "Canary",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            """"canary":"$n","target":"${it.substringAfter("-")}"""",
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                    } else if (it.length == 8) {
//                        module.log("@@@@@@@@$it")
                        n = n.drop(8)
                        n = it + n
                    }
//                    else{
//                        module.log("@@@@@@@@X$it")
//                    }
                }
                val oldRunnable = callback.args[0] as Runnable?
                oldRunnable?.let {
                    val targetClassName = "StackTracer_$n"
                    val wrapperRunnableClass = try {
                        globalClassLoader.loadClass(targetClassName)!!
                    } catch (_: Throwable) {
                        val classBytes = createRunnableTracerFunction(n)
                        val classLoader = InMemoryDexClassLoader(
                            ByteBuffer.wrap(classBytes),
                            globalClassLoader
                        )
                        classLoader.loadClass(targetClassName)
                    }

                    val runnableInstance = wrapperRunnableClass.getConstructor(Runnable::class.java)
                        .newInstance(Runnable {
                            it.run()
                        }) as Runnable
                    callback.args[0] = runnableInstance
                }



                return TraceThreadHookFirst(Random.nextInt())
            }

        }
    }

    @XposedHooker
    class TraceThreadHookSecond(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): TraceThreadHookSecond {
                var n = "${android.os.Process.myPid()}_${android.os.Process.myTid()}".padStart(
                    23,
                    '_'
                )
                canary()?.let {
                    if (it[1] == '-') {
                        n = n.drop(8)
                        n = it[0] + getRandomString(7) + n
                        module.sendDataToServer(
                            "Canary",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            """"canary":"$n","target":"${it.substringAfter("-")}"""",
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                    } else if (it.length == 8) {
//                        module.log("@@@@@@@@$it")
                        n = n.drop(8)
                        n = it + n
                    }
//                    else{
//                        module.log("@@@@@@@@X$it")
//                    }
                }
                val oldRunnable = callback.args[1] as Runnable?
                oldRunnable?.let {
                    val targetClassName = "StackTracer_$n"
                    val wrapperRunnableClass = try {
                        globalClassLoader.loadClass(targetClassName)!!
                    } catch (_: Throwable) {
                        val classBytes = createRunnableTracerFunction(n)
                        val classLoader = InMemoryDexClassLoader(
                            ByteBuffer.wrap(classBytes),
                            globalClassLoader
                        )
                        classLoader.loadClass(targetClassName)
                    }

                    val runnableInstance = wrapperRunnableClass.getConstructor(Runnable::class.java)
                        .newInstance(Runnable {
                            it.run()
                        }) as Runnable
                    callback.args[1] = runnableInstance
                }



                return TraceThreadHookSecond(Random.nextInt())
            }

        }
    }

    @XposedHooker
    class TraceCalableHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): TraceCalableHook {
                var n = "${android.os.Process.myPid()}_${android.os.Process.myTid()}".padStart(
                    23,
                    '_'
                )
                canary()?.let {
                    if (it[1] == '-') {
                        n = n.drop(8)
                        n = it[0] + getRandomString(7) + n
                        module.sendDataToServer(
                            "Canary",
                            callback.thisObject!!.javaClass.name,
                            callback.member.name,
                            """"canary":"$n","target":"${it.substringAfter("-")}"""",
                            callback.thisObject?.hashCode() ?: 0,
                            b64st()
                        )
                    } else if (it.length == 8) {
                        n = n.drop(8)
                        n = it + n
                    }
                }
                val targetClassName = "StackTracer_$n"
                val wrapperRunnableClass = try {
                    globalClassLoader.loadClass(targetClassName)!!
                } catch (_: Throwable) {
                    val classBytes = createCallableTracerFunction(n)
                    val classLoader = InMemoryDexClassLoader(
                        ByteBuffer.wrap(classBytes),
                        globalClassLoader
                    )
                    classLoader.loadClass(targetClassName)
                }
                val oldRunnable = callback.args[0] as Callable<*>
                val runnableInstance = wrapperRunnableClass.getConstructor(Callable::class.java)
                    .newInstance(Callable {
                        oldRunnable.call()
                    }) as Callable<*>
                callback.args[0] = runnableInstance


                return TraceCalableHook(Random.nextInt())
            }

        }
    }
    @XposedHooker
    class OnRegisterReceiver(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): OnRegisterReceiver {

                var actions = ""
                for (action in (callback.args[1] as IntentFilter).actionsIterator()){
                    if (actions.isNotEmpty()) {
                        actions += ","
                    }
                    actions += """"$action""""
                }
                val clazz = callback.args[0]?.javaClass?.name ?: ""
                actions = "[$actions]"
                val out = """"register_class":"$clazz","actions":$actions"""

                if (!module.registeredReceivers.contains(out)) {
                    module.registeredReceivers.add(out)
                    module.sendDataToServer(
                        "RegisterReceiver",
                        callback.thisObject!!.javaClass.name,
                        callback.member.name,
                        out,
                        callback.thisObject?.hashCode() ?: 0,
                        b64st()
                    )
                }

                return OnRegisterReceiver(Random.nextInt())
            }

        }
    }
    @XposedHooker
    class GenericHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): GenericHook {
                module.sendDataToServer(
                    "GenericHook",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    "",
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

                return GenericHook(Random.nextInt())
            }

        }
    }

    @XposedHooker
    class GetDeviceIDHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @SuppressLint("HardwareIds")
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): GetDeviceIDHook {
                val context = callback.args[0] as ContentResolver
                if (!module.isDeviceIdSent) {
                    module.isDeviceIdSent = true
                    module.sendDataToServer(
                        "DeviceID",
                        "android.provider.Settings.Secure",
                        callback.member.name,
                        """"device_id":"${Secure.getString(context, Secure.ANDROID_ID)}"""",
                        callback.thisObject?.hashCode() ?: 0,
                        b64st()
                    )
                }

                return GetDeviceIDHook(Random.nextInt())
            }

        }
    }

    @XposedHooker
    class SocketHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): SocketHook {
                return SocketHook(Random.nextInt())
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: SocketHook) {
                val socket = callback.result as Socket?
                var localAddr = ""
                var remoteAddr = ""
                var canary = ""
                socket?.localSocketAddress?.let {
                    localAddr = it.toString()
                }
                socket?.remoteSocketAddress?.let {
                    remoteAddr = it.toString()
                }
                canary()?.let {
                    if (it.length == 8) {
                        canary = it
                    }
                }
                val out =
                    """"local_addr": "$localAddr", "remote_addr": "$remoteAddr","canary":"$canary" """
                module.sendDataToServer(
                    "SocketHook",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

            }
        }
    }

    @XposedHooker
    class StartThreadHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): StartThreadHook {
                return StartThreadHook(Random.nextInt())
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: StartThreadHook) {
                val thread = callback.thisObject!! as Thread

                val out = """"new_jtid": ${thread.id}"""
                module.sendDataToServer(
                    "StartThreadHook",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )

            }
        }
    }

    @XposedHooker
    class ConnectHook(private val magic: Int) : XposedInterface.Hooker {
        companion object {
            @JvmStatic
            @BeforeInvocation
            fun beforeInvocation(callback: BeforeHookCallback): ConnectHook {
                return ConnectHook(Random.nextInt())
            }

            @JvmStatic
            @AfterInvocation
            fun afterInvocation(callback: AfterHookCallback, context: ConnectHook) {
                val socket = callback.thisObject as Socket?
                var localAddr = ""
                var remoteAddr = ""
                var canary = ""
                socket?.localSocketAddress?.let {
                    localAddr = it.toString()
                }
                socket?.remoteSocketAddress?.let {
                    remoteAddr = it.toString()
                }
                canary()?.let {
                    if (it.length == 8) {
                        canary = it
                    }
                }
                val out =
                    """"local_addr": "$localAddr", "remote_addr": "$remoteAddr","canary":"$canary" """
                module.sendDataToServer(
                    "SocketHook",
                    callback.thisObject!!.javaClass.name,
                    callback.member.name,
                    out,
                    callback.thisObject?.hashCode() ?: 0,
                    b64st()
                )
            }
        }
    }


    @SuppressLint("DiscouragedPrivateApi", "PrivateApi", "SoonBlockedPrivateApi")
    override fun onPackageLoaded(param: PackageLoadedParam) {
        super.onPackageLoaded(param)
        log("onPackageLoaded: " + param.packageName)
        log("param classloader is " + param.classLoader)
        log("module apk path: " + this.applicationInfo.sourceDir)
        log("----------")
        globalClassLoader = param.classLoader

//        if (!param.isFirstPackage) return

//        val prefs = getRemotePreferences("test")
//        log("remote prefs: " + prefs.getInt("test", -1))
//        log("before reg")
//        prefs.registerOnSharedPreferenceChangeListener { _, key ->
//            val value = prefs.getInt(key, 0)
//            log("onSharedPreferenceChanged: $key->$value")
//        }
//        mhook(Thread::class.java.getDeclaredMethod("start"), StartThreadHook::class.java)

//        private String android_id = Secure.getString(getContext().getContentResolver(),
//            Secure.ANDROID_ID);

        hook(
            Secure::class.java.getMethod(
                "getString",
                ContentResolver::class.java,
                String::class.java
            ), GetDeviceIDHook::class.java
        )

        var traceList = ScheduledThreadPoolExecutor::class.java.declaredMethods
        traceList += AbstractExecutorService::class.java.declaredMethods
        traceList += ThreadPoolExecutor::class.java.declaredMethods
        traceList += CompletableFuture::class.java.declaredMethods
        traceList += ForkJoinPool::class.java.declaredMethods
        traceList += Message::class.java.declaredMethods
        for (m in traceList) {
            if (m.parameterCount >= 1) {
                m.parameterTypes[0]?.let {
                    if (it.name == "java.lang.Runnable") {
                        hook(m, TraceThreadHookFirst::class.java)
                    }
                    if (it.name == "java.util.concurrent.Callable") {
                        hook(m, TraceCalableHook::class.java)
                    }
                }
            }
            if (m.parameterCount >= 2) {
                m.parameterTypes[1]?.let {
                    if (it.name == "java.lang.Runnable") {
                        hook(m, TraceThreadHookSecond::class.java)
                    }
                }
            }
        }
        var traceConstructors = FutureTask::class.java.declaredConstructors
        traceConstructors += Thread::class.java.declaredConstructors

        for (m in traceConstructors) {
            if (m.parameterCount >= 1) {
                m.parameterTypes[0]?.let {
                    if (it.name == "java.lang.Runnable") {
                        hook(m, TraceThreadHookFirst::class.java)
                    }
                    if (it.name == "java.util.concurrent.Callable") {
                        hook(m, TraceCalableHook::class.java)
                    }
                }
            }
            if (m.parameterCount >= 2) {
                m.parameterTypes[1]?.let {
                    if (it.name == "java.lang.Runnable") {
                        hook(m, TraceThreadHookSecond::class.java)
                    }
                }
            }
        }
        for (m in Handler::class.java.methods) {
            if (m.name.startsWith("post")) {
                hook(m, HandlerPostHook::class.java)
            }
        }

        for (m in Socket::class.java.methods) {
            if (m.name == "connect") {
                hook(m, ConnectHook::class.java)
            }
        }
        for (c in Socket::class.java.declaredConstructors) {
            hook(c, SocketHook::class.java)
        }


//        val systemJobSchedulerClass = Class.forName(
//            "androidx.work.impl.background.systemjob.SystemJobScheduler",
//            false,
//            param.classLoader
//        )
//        for (m in systemJobSchedulerClass.declaredMethods) {
//            log("method_name = ${m.toGenericString()}")
//            if (m.name == "scheduleInternal") {
//                mhook(m, ScheduleInternalHook::class.java)
//                break
//            }
//        }

//        val scheduleInternalMethod = SystemJobSchedulerClass::class.java.getDeclaredMethod(
//            "scheduleInternal",
//            WorkSpecClass::class.java,
//            Integer.TYPE,
//        )
//        mhook(scheduleInternalMethod, ScheduleInternalHook::class.java)


        // init(int opmode, Certificate certificate)
        val initIntCertificateMethod =
            Cipher::class.java.getDeclaredMethod("init", Integer.TYPE, Certificate::class.java)
        hook(initIntCertificateMethod, InitIntCertificateHook::class.java)
        // init(int opmode, Certificate certificate, SecureRandom random)
        val initIntCertificateSecureRandomMethod = Cipher::class.java.getDeclaredMethod(
            "init",
            Integer.TYPE,
            Certificate::class.java,
            SecureRandom::class.java
        )
        hook(initIntCertificateSecureRandomMethod, InitIntCertificateSecureRandomHook::class.java)
        // init(int opmode, Key key)
        val initIntKeyMethod =
            Cipher::class.java.getDeclaredMethod("init", Integer.TYPE, Key::class.java)
        hook(initIntKeyMethod, InitIntKeyHook::class.java)
        // init(int opmode, Key key, AlgorithmParameters params)
        val initIntKeyAlgorithmParametersMethod = Cipher::class.java.getDeclaredMethod(
            "init",
            Integer.TYPE,
            Key::class.java,
            AlgorithmParameters::class.java
        )
        hook(initIntKeyAlgorithmParametersMethod, InitIntKeyAlgorithmParametersHook::class.java)
        // init(int opmode, Key key, AlgorithmParameterSpec params)
        val initIntKeyAlgorithmParameterSpecMethod = Cipher::class.java.getDeclaredMethod(
            "init",
            Integer.TYPE,
            Key::class.java,
            AlgorithmParameterSpec::class.java
        )
        hook(
            initIntKeyAlgorithmParameterSpecMethod,
            InitIntKeyAlgorithmParameterSpecHook::class.java
        )
        // init(int opmode, Key key, AlgorithmParameterSpec params, SecureRandom random)
        val initIntKeyAlgorithmParameterSpecSecureRandomMethod =
            Cipher::class.java.getDeclaredMethod(
                "init",
                Integer.TYPE,
                Key::class.java,
                AlgorithmParameterSpec::class.java,
                SecureRandom::class.java
            )
        hook(
            initIntKeyAlgorithmParameterSpecSecureRandomMethod,
            InitIntKeyAlgorithmParameterSpecSecureRandomHook::class.java
        )
        // init(int opmode, Key key, AlgorithmParameters params, SecureRandom random)
        val initIntKeyAlgorithmParametersSecureRandomMethod = Cipher::class.java.getDeclaredMethod(
            "init",
            Integer.TYPE,
            Key::class.java,
            AlgorithmParameters::class.java,
            SecureRandom::class.java
        )
        hook(
            initIntKeyAlgorithmParametersSecureRandomMethod,
            InitIntKeyAlgorithmParametersSecureRandomHook::class.java
        )
        // init(int opmode, Key key, SecureRandom random)
        val initIntKeySecureRandomMethod = Cipher::class.java.getDeclaredMethod(
            "init",
            Integer.TYPE,
            Key::class.java,
            SecureRandom::class.java
        )
        hook(initIntKeySecureRandomMethod, InitIntKeySecureRandomHook::class.java)
        // update(byte[] input)
        val updateByteMethod = Cipher::class.java.getDeclaredMethod("update", ByteArray::class.java)
        hook(updateByteMethod, UpdateByteHook::class.java)
        // update(byte[] input, int inputOffset, int inputLen)
        val updateByteIntIntMethod = Cipher::class.java.getDeclaredMethod(
            "update",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE
        )
        hook(updateByteIntIntMethod, UpdateByteIntIntHook::class.java)
        // update(byte[] input, int inputOffset, int inputLen, byte[] output)
        val updateByteIntIntByteMethod = Cipher::class.java.getDeclaredMethod(
            "update",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE,
            ByteArray::class.java
        )
        hook(updateByteIntIntByteMethod, UpdateByteIntIntByteHook::class.java)
        // update(byte[] input, int inputOffset, int inputLen, byte[] output, int outputOffset)
        val updateByteIntIntByteIntMethod = Cipher::class.java.getDeclaredMethod(
            "update",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE,
            ByteArray::class.java,
            Integer.TYPE
        )
        hook(updateByteIntIntByteIntMethod, UpdateByteIntIntByteIntHook::class.java)
        // update(ByteBuffer input, ByteBuffer output)
        val updateByteBufferByteBufferMethod = Cipher::class.java.getDeclaredMethod(
            "update",
            ByteBuffer::class.java,
            ByteBuffer::class.java
        )
        hook(updateByteBufferByteBufferMethod, UpdateByteBufferByteBufferHook::class.java)
        // updateAAD(byte[] src)
        val updateAADByteMethod =
            Cipher::class.java.getDeclaredMethod("updateAAD", ByteArray::class.java)
        hook(updateAADByteMethod, UpdateAADByteHook::class.java)
        // updateAAD(byte[] src, int offset, int len)
        val updateAADByteIntIntMethod = Cipher::class.java.getDeclaredMethod(
            "updateAAD",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE
        )
        hook(updateAADByteIntIntMethod, UpdateAADByteIntIntHook::class.java)
        // updateAAD(ByteBuffer src)
        val updateAADByteBufferMethod =
            Cipher::class.java.getDeclaredMethod("updateAAD", ByteBuffer::class.java)
        hook(updateAADByteBufferMethod, UpdateAADByteBufferHook::class.java)
        // doFinal()
        val doFinalMethod = Cipher::class.java.getDeclaredMethod("doFinal")
        hook(doFinalMethod, DoFinalHook::class.java)
        // doFinal(byte[] input)
        val doFinalByteMethod =
            Cipher::class.java.getDeclaredMethod("doFinal", ByteArray::class.java)
        hook(doFinalByteMethod, DoFinalByteHook::class.java)
        // doFinal(byte[] output, int outputOffset)
        val doFinalByteIntMethod =
            Cipher::class.java.getDeclaredMethod("doFinal", ByteArray::class.java, Integer.TYPE)
        hook(doFinalByteIntMethod, DoFinalByteIntHook::class.java)
        // doFinal(byte[] input, int inputOffset, int inputLen)
        val doFinalByteIntIntMethod = Cipher::class.java.getDeclaredMethod(
            "doFinal",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE
        )
        hook(doFinalByteIntIntMethod, DoFinalByteIntIntHook::class.java)
        // doFinal(byte[] input, int inputOffset, int inputLen, byte[] output)
        val doFinalByteIntIntByteMethod = Cipher::class.java.getDeclaredMethod(
            "doFinal",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE,
            ByteArray::class.java
        )
        hook(doFinalByteIntIntByteMethod, DoFinalByteIntIntByteHook::class.java)
        // doFinal(byte[] input, int inputOffset, int inputLen, byte[] output,  int outputOffset)
        val doFinalByteIntIntByteIntMethod = Cipher::class.java.getDeclaredMethod(
            "doFinal",
            ByteArray::class.java,
            Integer.TYPE,
            Integer.TYPE,
            ByteArray::class.java,
            Integer.TYPE
        )
        hook(doFinalByteIntIntByteIntMethod, DoFinalByteIntIntByteIntHook::class.java)
        // doFinal(byte[] input, byte[] output)
        val doFinalByteByteMethod = Cipher::class.java.getDeclaredMethod(
            "doFinal",
            ByteBuffer::class.java,
            ByteBuffer::class.java
        )
        hook(doFinalByteByteMethod, DoFinalByteBufferByteBufferHook::class.java)


        // gps
        hook(Location::class.java.getDeclaredMethod("getLatitude"), LatitudeHook::class.java)
        hook(Location::class.java.getDeclaredMethod("getLongitude"), LongitudeHook::class.java)

        hook(
            BaseDexClassLoader::class.java.getDeclaredConstructor(
                String::class.java, File::class.java, String::class.java, ClassLoader::class.java
            ), DexLoaderHook::class.java
        )

        hook(
            Service::class.java.getDeclaredMethod(
                "startForeground",
                Integer.TYPE,
                Notification::class.java,
                Integer.TYPE,
            ), StartForegroundHook::class.java
        )
        hook(
            Service::class.java.getDeclaredMethod(
                "startForeground",
                Integer.TYPE,
                Notification::class.java,
            ), StartForegroundHook::class.java
        )


        hook(
            PendingIntent::class.java.getDeclaredMethod(
                "getService",
                Context::class.java,
                Integer.TYPE,
                Intent::class.java,
                Integer.TYPE
            ), PendingIntentGetHook::class.java
        )
        hook(
            PendingIntent::class.java.getDeclaredMethod(
                "getForegroundService",
                Context::class.java,
                Integer.TYPE,
                Intent::class.java,
                Integer.TYPE
            ), PendingIntentGetHook::class.java
        )
        hook(
            PendingIntent::class.java.getDeclaredMethod(
                "getBroadcast",
                Context::class.java,
                Integer.TYPE,
                Intent::class.java,
                Integer.TYPE
            ), PendingIntentGetHook::class.java
        )
//        module.hook(Handler::class.java.getDeclaredMethod("dispatchMessage", Message::class.java), GenericHook2::class.java)

        try {
            // Use reflection to access ActivityThread and get the system context
            val activityThreadClass = Class.forName("android.app.ActivityThread")
            val currentActivityThreadMethod: Method =
                activityThreadClass.getMethod("currentActivityThread")
            val currentActivityThread = currentActivityThreadMethod.invoke(null)
            val getSystemContextMethod: Method = activityThreadClass.getMethod("getSystemContext")
            val context = getSystemContextMethod.invoke(currentActivityThread) as Context
            for (it in context::class.java.methods) {
                if (!Modifier.isAbstract(it.modifiers)) {
                    if (it.name == "sendBroadcast" || it.name == "sendOrderedBroadcast" || it.name == "sendOrderedBroadcastAsUser" || it.name == "sendStickyBroadcast" || it.name == "sendStickyBroadcastAsUser" || it.name == "sendStickyOrderedBroadcast" || it.name == "sendStickyOrderedBroadcastAsUser" || it.name == "sendBroadcastAsUser" || it.name == "sendBroadcastWithMultiplePermissions" || it.name == "startService" || it.name == "startForegroundService") {
                        module.hook(it, FirstArgIntentHook::class.java)
                        continue
                    }
                    if (it.name == "bindService" || it.name == "bindServiceAsUser" || it.name == "bindIsolatedService") {
                        module.hook(it, BindServiceHook::class.java)
                    }
                    if (it.name == "registerReceiver"){
                        module.hook(it, OnRegisterReceiver::class.java)
                    }
                }
            }
            // Use the system context to get the JobScheduler service
            val jobScheduler =
                context.getSystemService(Context.JOB_SCHEDULER_SERVICE) as JobScheduler
            hook(
                jobScheduler::class.java.getDeclaredMethod("schedule", JobInfo::class.java),
                ScheduleHook::class.java
            )

            val alarmManager =
                context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
            alarmManager.let {
                module.hook(
                    it::class.java.getDeclaredMethod(
                        "setImpl",
                        Integer.TYPE,
                        Long::class.javaPrimitiveType,
                        Long::class.javaPrimitiveType,
                        Long::class.javaPrimitiveType,
                        Integer.TYPE,
                        PendingIntent::class.java,
                        AlarmManager.OnAlarmListener::class.java,
                        String::class.java,
                        Executor::class.java,
                        WorkSource::class.java,
                        AlarmManager.AlarmClockInfo::class.java,
                    ), SetImplHook::class.java
                )
            }
        } catch (e: Throwable) {
            log("Error: $e")
        }


        for (p in getDexFiles(param.applicationInfo.sourceDir)) {
            module.scanDexFile(p, param.classLoader)
        }
    }

    fun scanDexFile(dexPath: String, classLoader: ClassLoader) {
        if (module.addedDexFiles.contains(dexPath)) {
            return
        }
        module.addedDexFiles.add(dexPath)
        try {
            val classNames = getClassesFromClassLoader(classLoader)
            classNames.forEach { className ->
                try {
                    val clazz = classLoader.loadClass(className)
                    if (className == "androidx.work.impl.background.systemjob.SystemJobScheduler") {
                        for (m in clazz.declaredMethods) {
                            if (m.name == "scheduleInternal") {
                                hook(m, ScheduleInternalHook::class.java)
                                break
                            }
                        }
                    }else if (className.startsWith("kotlinx.coroutines.scheduling.")) {
                        for (m in clazz.declaredMethods) {
                            if (m.parameterCount >= 1) {
                                m.parameterTypes[0]?.let {
                                    if (it.name == "java.lang.Runnable") {
                                        hook(m, TraceThreadHookFirst::class.java)
                                    }
                                    if (it.name == "java.util.concurrent.Callable") {
                                        hook(m, TraceCalableHook::class.java)
                                    }
                                }
                            }
                            if (m.parameterCount >= 2) {
                                m.parameterTypes[1]?.let {
                                    if (it.name == "java.lang.Runnable") {
                                        hook(m, TraceThreadHookSecond::class.java)
                                    }
                                }
                            }
                        }
                    } else if (className == "androidx.work.WorkRequest") {
                        for (m in clazz.declaredConstructors) {
                            hook(m, WorkRequestHook::class.java)
                        }
                    } else if (!Modifier.isAbstract(clazz.modifiers)) { // Worker::class.java.isAssignableFrom(clazz) &&
//                        for (m in getSendBroadcastMethods(clazz)) {
//                            module.hook(m, FirstArgIntentHook::class.java)
////                            log("Found send broadcast implementation: ${clazz.name}")
//                        }
//                        getMethodClass(
//                            clazz,
//                            "android.content.Context",
//                            "startService",
//                            Intent::class.java
//                        )?.let {
//                            module.hook(it, FirstArgIntentHook::class.java)
////                            log("Found start service implementation: ${clazz.name}")
//                        }
//                        getMethodClass(
//                            clazz,
//                            "android.content.Context",
//                            "startForegroundService",
//                            Intent::class.java
//                        )?.let {
//                            module.hook(it, FirstArgIntentHook::class.java)
////                            log("Found start foreground service implementation: ${clazz.name}")
//                        }
//                        getMethodClass(
//                            clazz, "android.content.Context", "bindService",
//                            Intent::class.java,
//                            ServiceConnection::class.java,
//                            Integer.TYPE
//                        )?.let {
//                            module.hook(it, BindServiceHook::class.java)
////                            log("Found bind service implementation: ${clazz.name}")
//                        }

                        getMethodClass(clazz, "android.app.Service", "onCreate")?.let { m ->
                            module.hook(m, GenericHook::class.java)
                            getMethodClass(
                                clazz,
                                "android.app.Service",
                                "onBind",
                                Intent::class.java
                            )?.let {
                                module.hook(it, OnBindHook::class.java)
                            }
                            getMethodClass(
                                clazz,
                                "android.app.Service",
                                "onRebind",
                                Intent::class.java
                            )?.let {
                                module.hook(it, OnBindHook::class.java)
                            }
                            getMethodClass(
                                clazz, "android.app.Service",
                                "onStartCommand",
                                Intent::class.java,
                                Integer.TYPE,
                                Integer.TYPE
                            )?.let {
                                module.hook(it, OnStartCommandHook::class.java)
                            }
                        }
                        getMethodClass(
                            clazz,
                            "android.app.AlarmManager.OnAlarmListener",
                            "onAlarm"
                        )?.let {
                            module.hook(it, GenericHook::class.java)
//                            log("Found Alarm implementation: ${clazz.name}")
                        }

                        getMethodClass(
                            clazz,
                            "android.app.job.JobService",
                            "onStartJob",
                            JobParameters::class.java
                        )?.let {
                            module.hook(it, OnStartJobHook::class.java)
                        }
                        getMethodClass(clazz, "androidx.work.Worker", "doWork")?.let {
                            module.hook(it, DoWorkHook::class.java)
                        }
                        getMethodClass(
                            clazz,
                            "android.content.BroadcastReceiver",
                            "onReceive",
                            Context::class.java,
                            Intent::class.java
                        )?.let {
                            module.hook(it, OnReceiveHook::class.java)
                        }
////                            classCount++
                    }
                } catch (_: Throwable) {
//                    log("Error hooking class $className: ${e.message}")
                }
            }
        } catch (_: Throwable) {
//            log("Error loading classes using DexFile: ${e.message}")
        }
    }

    @SuppressLint("DiscouragedPrivateApi")
    private fun getClassesFromClassLoader(classLoader: ClassLoader): List<String> {
        val classNames = mutableListOf<String>()
        try {
            val pathListField = BaseDexClassLoader::class.java.getDeclaredField("pathList")
            pathListField.isAccessible = true
            val pathListObject = pathListField.get(classLoader)
            val dexElementsField = pathListObject::class.java.getDeclaredField("dexElements")
            dexElementsField.isAccessible = true
            val dexElementsArray = dexElementsField.get(pathListObject) as Array<*>
            for (element in dexElementsArray) {
                val dexFileField = element!!::class.java.getDeclaredField("dexFile")
                dexFileField.isAccessible = true
                val dexFileObject = dexFileField.get(element) ?: continue
                val dexClassNamesMethod = dexFileObject::class.java.getDeclaredMethod("entries")
                dexClassNamesMethod.isAccessible = true
                val enumeration = dexClassNamesMethod.invoke(dexFileObject) as Enumeration<*>
                while (enumeration.hasMoreElements()) {
                    classNames.add(enumeration.nextElement() as String)
                }
            }
        } catch (_: Throwable) {
//            log("Error retrieving classes from class loader: ${e.message}")
        }
        return classNames
    }
//    private inline fun hook(origin: Method, hooker: Class<out Hooker?>): MethodUnhooker<Method> {
//        module.log("===> ${origin.javaClass}->${origin.name}")
//        return hook(origin, hooker)
//    }
//    private inline fun mhook(origin: Constructor<*>, hooker: Class<out Hooker?>): MethodUnhooker<out Constructor<*>> {
//        module.log("===> ${origin.javaClass}->${origin.name}")
//        return hook(origin, hooker)
//    }
}

private fun getDexFiles(apkPath: String): List<String> {
    val dexFiles = mutableListOf<String>()
    try {
        dexFiles.add(apkPath)
        val apkFile = File(apkPath)
        val oDexDir = File(apkFile.parentFile, "oat")
        if (oDexDir.exists() && oDexDir.isDirectory) {
            dexFiles.addAll(oDexDir.listFiles { file ->
                file.extension == "dex" || file.extension == "odex"
            }?.map { it.absolutePath } ?: emptyList())
        }
    } catch (e: Throwable) {
//        log("Error finding additional dex files: ${e.message}")
    }
    return dexFiles
}

//private fun getOnCreateMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod(
//            "onCreate",
//        )
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.app.Service") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getOnStartCommandMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod(
//            "onStartCommand",
//            Intent::class.java,
//            Integer.TYPE,
//            Integer.TYPE
//        )
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.app.Service") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}


private fun getMethodClass(
    clazz: Class<*>,
    className: String,
    methodName: String,
    vararg parameterTypes: Class<*>?
): Method? {
    return try {
        val m = clazz.getDeclaredMethod(
            methodName,
            *parameterTypes,
        )
        var currentClass: Class<*>? = clazz
        while (currentClass != null) {
            if (currentClass.name == className) {
                return m
            }
            currentClass = currentClass.superclass
        }
        null
    } catch (_: Throwable) {
        null
    }
}

//private fun getOnOnBindMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod(
//            "onBind",
//            Intent::class.java,
//        )
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.app.Service") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

private fun getSendBroadcastMethods(clazz: Class<*>): Array<Method> {
    val methods = mutableListOf<Method>()
    try {
        var currentClass: Class<*>? = clazz

        while (currentClass != null) {
            if (currentClass.name == "android.content.Context") {
                methods.addAll(
                    clazz.declaredMethods.filter {
                        Modifier.isAbstract(it.modifiers) &&
                                (it.name == "sendBroadcast" || it.name == "sendOrderedBroadcast" || it.name == "sendOrderedBroadcastAsUser" || it.name == "sendStickyBroadcast" || it.name == "sendStickyBroadcastAsUser" || it.name == "sendStickyOrderedBroadcast" || it.name == "sendStickyOrderedBroadcastAsUser" || it.name == "sendBroadcastAsUser" || it.name == "sendBroadcastWithMultiplePermissions")
                    }
                )
                break
            }
            currentClass = currentClass.superclass
        }
    } catch (_: Throwable) {

    }

    return methods.toTypedArray()
}

private fun isExecutorClass(clazz: Class<*>): Boolean {

    try {
        var currentClass: Class<*>? = clazz

        while (currentClass != null) {
            if (currentClass.name.startsWith("java.util.concurrent") && !currentClass.name.startsWith(
                    "java.util.concurrent.atomic"
                ) && currentClass.name != "java.util.concurrent.CancellationException"
            ) {
                return true
            }
            currentClass = currentClass.superclass
        }
    } catch (_: Throwable) {

    }
    return false
}

//private fun threadStartMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("start")
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "java.lang.Thread") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getBindServiceMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod(
//            "bindService",
//            Intent::class.java,
//            ServiceConnection::class.java,
//            Integer.TYPE
//        )
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.content.Context") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getStartForegroundServiceMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("startForegroundService", Intent::class.java)
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.content.Context") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getStartServiceMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("startService", Intent::class.java)
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.content.Context") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getOnStartJobMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("onStartJob", JobParameters::class.java)
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.app.job.JobService") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getDoWorkMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("doWork")
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "androidx.work.Worker") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}

//private fun getOnAlarmMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("onAlarm")
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.app.AlarmManager.OnAlarmListener") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}


//private fun getOnReceiveMethod(clazz: Class<*>): Method? {
//    return try {
//        val m = clazz.getDeclaredMethod("onReceive", Context::class.java, Intent::class.java)
//        var currentClass: Class<*>? = clazz
//        while (currentClass != null) {
//            if (currentClass.name == "android.content.BroadcastReceiver") { // || currentClass == Worker::class.java
//                return m
//            }
//            currentClass = currentClass.superclass
//        }
//        null
//    } catch (_: Throwable) { // (e: NoSuchMethodException)
//        null
//    }
//}


//@Suppress("NOTHING_TO_INLINE")
//private inline fun b64st(): String {
//    // Get the stack trace as a string
//    val stackTraceString = Thread.currentThread().stackTrace.joinToString("\n")
//
//    // Compress the stack trace using BZIP2
//    val compressedStackTrace = ByteArrayOutputStream().use { byteStream ->
//        BZip2CompressorOutputStream(byteStream).use { bzip2Stream ->
//            bzip2Stream.write(stackTraceString.toByteArray())
//        }
//        byteStream.toByteArray()
//    }
//
//    // Encode the compressed stack trace to Base64
//    return Base64.getEncoder().encodeToString(compressedStackTrace)
//}

@Suppress("NOTHING_TO_INLINE")
private inline fun canary(): String? {
    // Get the stack trace as a string
    for (row in Thread.currentThread().stackTrace) {
        val stackTraceString = row.toString()
        if (stackTraceString.contains("StackTracer_") && !stackTraceString.contains("StackTracer__")) {
            val startIndex = stackTraceString.indexOf("StackTracer_") + "StackTracer_".length
            return stackTraceString.substring(startIndex, startIndex + 8)
        }

        if (stackTraceString.contains("LSPHooker_") && stackTraceString.contains(".onReceive(")) {
            return "1-"+stackTraceString.substringBefore("(").substringAfter("_")
        }
        if (stackTraceString.startsWith("LSPHooker_") && stackTraceString.contains(".doWork(")) {
            return "2-"+stackTraceString.substringBefore("(").substringAfter("_")
        }
        if (stackTraceString.contains("LSPHooker_") && stackTraceString.contains(".onStartJob(")) {
            return "3-"+stackTraceString.substringBefore("(").substringAfter("_")
        }
        if (stackTraceString.contains("LSPHooker_") && stackTraceString.contains(".onStartCommand(")) {
            return "4-"+stackTraceString.substringBefore("(").substringAfter("_")
        }
        if (stackTraceString.contains("LSPHooker_") && stackTraceString.contains(".onBind(")) {
            return "5-"+stackTraceString.substringBefore("(").substringAfter("_")
        }
        if (stackTraceString.contains("LSPHooker_") && stackTraceString.contains(".onRebind(")) {
            return "6-"+stackTraceString.substringBefore("(").substringAfter("_")
        }
        if (stackTraceString.contains("LSPHooker_") && stackTraceString.contains(".onCreate(")) {
            return "7-"+stackTraceString.substringBefore("(").substringAfter("_")
        }

    }
//    val stackTraceString = Thread.currentThread().stackTrace.joinToString("\n")

    // Encode the compressed stack trace to Base64
    return null
}

@Suppress("NOTHING_TO_INLINE")
private inline fun b64st(): String {
    // Get the stack trace as a string
    val stackTraceString = Thread.currentThread().stackTrace.joinToString("\n")

    // Compress the stack trace using GZIP
    val compressedStackTrace = ByteArrayOutputStream().use { byteStream ->
        GZIPOutputStream(byteStream).use { gzipStream ->
            gzipStream.write(stackTraceString.toByteArray())
        }
        byteStream.toByteArray()
    }

    // Encode the compressed stack trace to Base64
    return Base64.getEncoder().encodeToString(compressedStackTrace)
}

fun getRandomString(length: Int): String {
    val allowedChars = ('A'..'Z') + ('a'..'z') + ('0'..'9')
    return (1..length)
        .map { allowedChars.random() }
        .joinToString("")
}


private fun createRunnableTracerFunction(name: String): ByteArray {
    val magic = Base64.getDecoder().decode("ZGV4CjAzNQA=")
    val part1 = Base64.getDecoder()
        .decode("XAQAAHAAAAB4VjQSAAAAAAAAAACYAwAAEgAAAHAAAAAHAAAAuAAAAAMAAADUAAAAAQAAAPgAAAAFAAAAAAEAAAEAAAAoAQAAFAMAAEgBAAC6AQAAwgEAAMYBAADJAQAAzQEAAPQBAAATAgAAKwIAAEMCAABXAgAAbQIAAJcCAACaAgAAngIAAKkCAACsAgAAsQIAALgCAAACAAAABAAAAAUAAAAGAAAACAAAAAkAAAALAAAAAwAAAAAAAACsAQAACwAAAAYAAAAAAAAADAAAAAYAAAC0AQAAAQAFAA4AAAABAAIAAAAAAAEAAAANAAAAAQABAA8AAAAEAAEAAAAAAAUAAQAPAAAAAQAAAAEAAAAEAAAApAEAAAoAAACIAwAAaAMAAAAAAAACAAIAAAAAAAAAAAACAAAAEvEPAQIAAgABAAAAmAEAAAYAAABwEAMAAABbAQAADgACAAEAAQAAAJ4BAAAIAAAAVBAAADgABQByEAQAAAAOAAQBAA48AAoADksAAAIAAAAFAAMAAQAAAAQAAAABAAAABQAGPGluaXQ+AAI+OwABSQACSUwAJUxTdGFja1RyYWNlcl8=")!!
    val part2 = Base64.getDecoder()
        .decode("OwAdTGRhbHZpay9hbm5vdGF0aW9uL1NpZ25hdHVyZTsAFkxqYXZhL2xhbmcvQ29tcGFyYWJsZTsAFkxqYXZhL2xhbmcvQ29tcGFyYWJsZTwAEkxqYXZhL2xhbmcvT2JqZWN0OwAUTGphdmEvbGFuZy9SdW5uYWJsZTsAKFN0YWNrVHJhY2VyXw==")!!
    val part3 = Base64.getDecoder()
        .decode("LmphdmEAAVYAAlZMAAljb21wYXJlVG8AAXIAA3J1bgAFdmFsdWUAnQF+fkQ4eyJiYWNrZW5kIjoiZGV4IiwiY29tcGlsYXRpb24tbW9kZSI6InJlbGVhc2UiLCJoYXMtY2hlY2tzdW1zIjpmYWxzZSwibWluLWFwaSI6MSwic2hhLTEiOiJmYWNlZGY0MWJiZDI4YjU2M2QxZTllMDljNWY3MmQ3YzVjYTU5OGQ1IiwidmVyc2lvbiI6IjguMi4yLWRldiJ9AAICARAcBRcIFwkXBxcIFwEAAQECABIAgYAE3AIBAcgCAQH4AgAAAAABAAAAWAMAAIADAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAEAAAAAAAAAAQAAABIAAABwAAAAAgAAAAcAAAC4AAAAAwAAAAMAAADUAAAABAAAAAEAAAD4AAAABQAAAAUAAAAAAQAABgAAAAEAAAAoAQAAASAAAAMAAABIAQAAAyAAAAIAAACYAQAAARAAAAMAAACkAQAAAiAAABIAAAC6AQAABCAAAAEAAABYAwAAACAAAAEAAABoAwAAAxAAAAIAAAB8AwAABiAAAAEAAACIAwAAABAAAAEAAACYAwAA")!!
    var total = part1 + name.toByteArray() + part2 + name.toByteArray() + part3
    val digest = MessageDigest.getInstance("SHA-1")
    total = digest.digest(total) + total
    val checksum = Adler32()
    checksum.update(total)
    val byteChecksum = ByteBuffer.allocate(Int.SIZE_BYTES)
    byteChecksum.order(ByteOrder.LITTLE_ENDIAN)
    byteChecksum.putInt(checksum.value.toInt())
    total = magic + byteChecksum.array() + total
    return total
}

private fun createCallableTracerFunction(name: String): ByteArray {
    val magic = Base64.getDecoder().decode("ZGV4CjAzNQA=")
    val part1 = Base64.getDecoder()
        .decode("3AQAAHAAAAB4VjQSAAAAAAAAAAAYBAAAFgAAAHAAAAAHAAAAyAAAAAMAAADkAAAAAQAAAAgBAAAEAAAAEAEAAAEAAAAwAQAAjAMAAFABAACmAQAAqQEAALABAAC1AQAAvQEAAMABAADDAQAA6gEAAAkCAAAlAgAAPAIAAFACAABxAgAAkgIAALwCAADDAgAAzAIAAM8CAADTAgAA1gIAANwCAADjAgAABgAAAAcAAAAIAAAACQAAAAoAAAALAAAAEAAAAAUAAAAEAAAAAAAAABAAAAAGAAAAAAAAABEAAAAGAAAAoAEAAAAABQASAAAAAAACAAMAAAAAAAAAEwAAAAQAAQADAAAABQAAABMAAAAAAAAAAQAAAAQAAACgAQAADQAAAPADAAC7AwAAAAAAAAIAAQABAAAAlAEAAAsAAABUEAAAOAAHAHIQAwAAAAwAEQASABEAAAACAAIAAQAAAJkBAAAGAAAAcBACAAAAWwEAAA4ADAAOSwAGAQAOPAAAAQAAAAUAASgABSgpVFY7AAM8VjoABjxpbml0PgABPgABTAAlTFN0YWNrVHJhY2VyXw==")!!
    val part2 = Base64.getDecoder()
        .decode("OwAdTGRhbHZpay9hbm5vdGF0aW9uL1NpZ25hdHVyZTsAGkxkYWx2aWsvYW5ub3RhdGlvbi9UaHJvd3M7ABVMamF2YS9sYW5nL0V4Y2VwdGlvbjsAEkxqYXZhL2xhbmcvT2JqZWN0OwAfTGphdmEvdXRpbC9jb25jdXJyZW50L0NhbGxhYmxlOwAfTGphdmEvdXRpbC9jb25jdXJyZW50L0NhbGxhYmxlPAAoU3RhY2tUcmFjZXJf")!!
    val part3 = Base64.getDecoder()
        .decode("LmphdmEABVRWOz47AAdUVjs+OylWAAFWAAJWTAABYwAEY2FsbAAFdmFsdWUAnQF+fkQ4eyJiYWNrZW5kIjoiZGV4IiwiY29tcGlsYXRpb24tbW9kZSI6InJlbGVhc2UiLCJoYXMtY2hlY2tzdW1zIjpmYWxzZSwibWluLWFwaSI6MSwic2hhLTEiOiJmYWNlZGY0MWJiZDI4YjU2M2QxZTllMDljNWY3MmQ3YzVjYTU5OGQ1IiwidmVyc2lvbiI6IjguMi4yLWRldiJ9AAIBARQcAxcAFwwXDwIBARQcARcBAgIBFBwBGAMCAQEUHAIXDBcOAgEBFBwGFwIXChcEFwoXDBcOAAEBAQASAIGABPgCAQHQAgABAAAAgwMAAAIAAACPAwAAlwMAAAEAAACfAwAAAQAAAKkDAADoAwAAAQAAAAIAAAAAAAAAAAAAAOADAAAAAAAAzAMAAAEAAADUAwAAEAAAAAAAAAABAAAAAAAAAAEAAAAWAAAAcAAAAAIAAAAHAAAAyAAAAAMAAAADAAAA5AAAAAQAAAABAAAACAEAAAUAAAAEAAAAEAEAAAYAAAABAAAAMAEAAAEgAAACAAAAUAEAAAMgAAACAAAAlAEAAAEQAAABAAAAoAEAAAIgAAAWAAAApgEAAAQgAAAFAAAAgwMAAAAgAAABAAAAuwMAAAMQAAAEAAAAzAMAAAYgAAABAAAA8AMAAAAQAAABAAAAGAQAAA==")!!
    var total = part1 + name.toByteArray() + part2 + name.toByteArray() + part3
    val digest = MessageDigest.getInstance("SHA-1")
    total = digest.digest(total) + total
    val checksum = Adler32()
    checksum.update(total)
    val byteChecksum = ByteBuffer.allocate(Int.SIZE_BYTES)
    byteChecksum.order(ByteOrder.LITTLE_ENDIAN)
    byteChecksum.putInt(checksum.value.toInt())
    total = magic + byteChecksum.array() + total
    return total
}

