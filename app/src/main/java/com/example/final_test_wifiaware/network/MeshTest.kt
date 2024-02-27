package com.example.final_test_wifiaware.network
/*
import android.content.Context
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.preferencesDataStore
import com.ustadmobile.meshrabiya.log.MNetLoggerStdout
import com.ustadmobile.meshrabiya.vnet.AndroidVirtualNode
import com.ustadmobile.meshrabiya.vnet.wifi.ConnectBand
import com.ustadmobile.meshrabiya.vnet.wifi.HotspotType
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class MeshTest(context: Context) {
    private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "meshr_settings")
    val myNode: AndroidVirtualNode

    init {
        myNode = AndroidVirtualNode(
            appContext = context,
            dataStore = context.dataStore,
            logger =  MNetLoggerStdout(minLogLevel = Log.DEBUG)
        )

        CoroutineScope(Dispatchers.Main).launch {
            myNode.setWifiHotspotEnabled(
                enabled = true,
                preferredBand = ConnectBand.BAND_5GHZ,
                hotspotType = HotspotType.WIFIDIRECT_GROUP
            )
        }
    }


}*/


import android.content.Context
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.preferencesDataStore
import com.ustadmobile.meshrabiya.log.MNetLoggerStdout
import com.ustadmobile.meshrabiya.vnet.AndroidVirtualNode
import com.ustadmobile.meshrabiya.vnet.wifi.ConnectBand
import com.ustadmobile.meshrabiya.vnet.wifi.HotspotType

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class MeshTest(context: Context) {
    private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "meshr_settings")
    val myNode: AndroidVirtualNode

    init {
        myNode = AndroidVirtualNode(
            appContext = context,
            dataStore = context.dataStore,
            logger =  MNetLoggerStdout(minLogLevel = Log.DEBUG)
        )

        CoroutineScope(Dispatchers.Main).launch {
            myNode.setWifiHotspotEnabled(
                enabled = true,
                preferredBand = ConnectBand.BAND_5GHZ,
                hotspotType = HotspotType.WIFIDIRECT_GROUP
            )
        }
    }


}
