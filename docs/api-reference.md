# API Reference

Packages:

- [kastlewatch.io/v1alpha1](#kastlewatchiov1alpha1)

# kastlewatch.io/v1alpha1

Resource Types:

- [TCPMonitor](#tcpmonitor)




## TCPMonitor
<sup><sup>[↩ Parent](#kastlewatchiov1alpha1 )</sup></sup>






Auto-generated derived type for TCPMonitorSpec via `CustomResource`

<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Description</th>
            <th>Required</th>
        </tr>
    </thead>
    <tbody><tr>
      <td><b>apiVersion</b></td>
      <td>string</td>
      <td>kastlewatch.io/v1alpha1</td>
      <td>true</td>
      </tr>
      <tr>
      <td><b>kind</b></td>
      <td>string</td>
      <td>TCPMonitor</td>
      <td>true</td>
      </tr>
      <tr>
      <td><b><a href="https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.27/#objectmeta-v1-meta">metadata</a></b></td>
      <td>object</td>
      <td>Refer to the Kubernetes API documentation for the fields of the `metadata` field.</td>
      <td>true</td>
      </tr><tr>
        <td><b><a href="#tcpmonitorspec">spec</a></b></td>
        <td>object</td>
        <td>
          Specification for the TCPMonitor resource<br/>
        </td>
        <td>true</td>
      </tr><tr>
        <td><b><a href="#tcpmonitorstatus">status</a></b></td>
        <td>object</td>
        <td>
          The status of the monitor resource<br/>
        </td>
        <td>false</td>
      </tr></tbody>
</table>


### TCPMonitor.spec
<sup><sup>[↩ Parent](#tcpmonitor)</sup></sup>



Specification for the TCPMonitor resource

<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Description</th>
            <th>Required</th>
        </tr>
    </thead>
    <tbody><tr>
        <td><b><a href="#tcpmonitorspechost_config">host_config</a></b></td>
        <td>object</td>
        <td>
          Configuration for the target host<br/>
        </td>
        <td>true</td>
      </tr><tr>
        <td><b><a href="#tcpmonitorspecmonitor_config">monitor_config</a></b></td>
        <td>object</td>
        <td>
          Configuration for the monitoring behavior<br/>
        </td>
        <td>true</td>
      </tr></tbody>
</table>


### TCPMonitor.spec.host_config
<sup><sup>[↩ Parent](#tcpmonitorspec)</sup></sup>



Configuration for the target host

<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Description</th>
            <th>Required</th>
        </tr>
    </thead>
    <tbody><tr>
        <td><b>host</b></td>
        <td>string</td>
        <td>
          The hostname or IP address of the target<br/>
        </td>
        <td>true</td>
      </tr><tr>
        <td><b>port</b></td>
        <td>integer</td>
        <td>
          The port number to check<br/>
          <br/>
            <i>Format</i>: uint16<br/>
            <i>Minimum</i>: 0<br/>
        </td>
        <td>true</td>
      </tr></tbody>
</table>


### TCPMonitor.spec.monitor_config
<sup><sup>[↩ Parent](#tcpmonitorspec)</sup></sup>



Configuration for the monitoring behavior

<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Description</th>
            <th>Required</th>
        </tr>
    </thead>
    <tbody><tr>
        <td><b>polling_frequency</b></td>
        <td>integer</td>
        <td>
          Frequency in seconds to poll the target<br/>
          <br/>
            <i>Format</i>: uint32<br/>
            <i>Minimum</i>: 0<br/>
        </td>
        <td>true</td>
      </tr><tr>
        <td><b>retries</b></td>
        <td>integer</td>
        <td>
          Number of retries before considering the check failed<br/>
          <br/>
            <i>Format</i>: uint32<br/>
            <i>Minimum</i>: 0<br/>
        </td>
        <td>true</td>
      </tr><tr>
        <td><b>timeout</b></td>
        <td>integer</td>
        <td>
          Timeout in seconds for the connection attempt<br/>
          <br/>
            <i>Format</i>: uint32<br/>
            <i>Minimum</i>: 0<br/>
        </td>
        <td>true</td>
      </tr></tbody>
</table>


### TCPMonitor.status
<sup><sup>[↩ Parent](#tcpmonitor)</sup></sup>



The status of the monitor resource

<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Description</th>
            <th>Required</th>
        </tr>
    </thead>
    <tbody><tr>
        <td><b>state</b></td>
        <td>enum</td>
        <td>
          The current state of the monitor<br/>
          <br/>
            <i>Enum</i>: Healthy, Warning, Critical, NoData<br/>
        </td>
        <td>true</td>
      </tr><tr>
        <td><b>last_checked</b></td>
        <td>string</td>
        <td>
          The timestamp of the last check in RFC3339 format<br/>
        </td>
        <td>false</td>
      </tr></tbody>
</table>
