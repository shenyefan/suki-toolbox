# 系统信息检测说明

Windows 汇总入口位于 `src/system_info/windows.rs`；各检测器按类别拆在 `src/system_info/windows/`，MI 属性读取边界位于 `src/system_info/mi.rs`。
本文记录实际使用的接口、字段、记录组合与虚拟设备判定，不代表专项硬件检测或完整资产盘点。

系统信息页使用 GPUI Kit 原生 `DropdownButton` 提供文本复制和“保存截图”。下拉菜单通过 `.dropdown_menu(|menu, _, _| menu.menu("保存截图", Box::new(SaveScreenshot)))` 注册标准 Action，沿用组件菜单外观，不额外嵌套菜单或添加自定义菜单项。“保存截图”按用户决定不加省略号。保存前通过 `prompt_for_new_path` 显示原生保存对话框，选择路径后从点击时的完整快照、运行时间和当前深色／浅色模式生成，取消不生成文件。`src/system_info/export.rs` 在后台使用 GDI 测量和绘制当前模式对应纯色主题的三张概览卡片与详细信息分组，类别与主屏／虚拟设备图标复用相同的 Lucide 资源和 GPUI SVG 栅格器；按完整内容扩展高度，将 PNG 写入所选路径。图片底部包含 Suki Toolbox 和 Cargo 版本，本地生成时间由 `GetLocalTime` 提供。导出不使用半透明材质，不读取屏幕或依赖滚动位置。

## 文件职责

| 文件（位于 `src/system_info/windows/`） | 职责 |
| --- | --- |
| `system.rs` | 机器型号、Windows 版本、运行时间和版本注册表属性 |
| `mainboard.rs` | 主板 |
| `processor.rs` | 处理器 |
| `memory.rs` | 内存与系统报告的已使用／总插槽数 |
| `graphics.rs` | 显卡枚举、实例身份组合、显存和虚拟类型 |
| `monitor.rs` | 显示路径、显示器名称和尺寸、当前模式、主屏及虚拟类型 |
| `disk.rs` | 磁盘 |
| `battery.rs` | 电池来源关联、容量和健康度 |
| `audio.rs` | 音频接口枚举、连续音频父子节点归并和声卡类型 |
| `network.rs` | 网络接口 |
| `pnp.rs` | 共用 PnP 身份、名称、GUID 属性和 WMI 实例名称匹配 |

## 共用规则

- WMI/CIM 查询通过 Windows MI API 执行，使用本机 `WMIDCOM` 会话与 WQL；每次查询的超时参数为 5 秒，支持取消。
- 查询在后台执行；结果由页面请求编号校验，通过 GPUI 上下文发布。错误使用 toast，已有主界面保持可用。
- 各类别统一按照已确认物理设备、类型未确认设备、已确认虚拟设备排列；同组内保留系统返回顺序。排序在汇总模型的统一入口执行，覆盖全部九个硬件类别，主屏属性不改变类型顺序。同名设备不会按名称合并；显卡按设备实例 ID 组合来源，声卡按设备实例及连续音频父子关系组合接口。
- 首选名称缺失时尝试其他可用名称；全部缺失时显示“未知设备”并保留记录、容量、类型及标记。设备名称不作为记录是否存在的条件，也不作为关联键。
- 名称、厂商、型号、容量和路径中的关键词不参与虚拟类型判断。字段缺失不等于物理设备；缺少判定属性时保留设备且不添加虚拟标注。
- 设备内容、主屏属性与虚拟类型分别保存，不从显示文字解析标记。虚拟类型为可空布尔属性：`true` 表示已确认虚拟，`false` 表示来源确认非虚拟，缺失表示未确认。每份快照为设备记录分配身份，排序和页面重绘时保持，供图标提示与可访问节点使用。
- 主屏和虚拟设备在所在设备行的最右侧固定区域显示不同图标，右对齐；图标含可访问名称和悬停提示，可同时显示。复制文本按相同顺序输出，并用“主屏”前缀与“（虚拟设备）”后缀保留含义。
- DXGI 软件渲染器、隐藏网络接口不在设备概览中展示。实际虚拟显示适配器、可见虚拟网络接口及虚拟声卡仍展示。
- “现代系统属性”按设备类别选用。主板、CPU、内存与电池没有可直接替换成 `MSFT_* .Virtual` 的通用接口，仍使用其实际提供字段的管理类，不臆造新类或统一虚拟属性。

## 全部检测来源

| 类别 | 接口与字段 | 组合方式 | 虚拟类型判定 |
| --- | --- | --- | --- |
| 机器型号 | `root/cimv2:Win32_ComputerSystem.Manufacturer, Model` | 同一记录的厂商与型号拼接 | 不推断；不以型号包含 Virtual、VMware 等文字判定 |
| Windows | `Win32_OperatingSystem.Caption, BuildNumber`；注册表 `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` 的 `DisplayVersion, UBR` | 系统名称、功能版本、内部版本与 UBR 修订号拼接 | 不适用 |
| 运行时间 | `GetTickCount64()` | 毫秒转换为秒，页面单独定时更新 | 不适用 |
| 主板 | `Win32_BaseBoard.Manufacturer, Product, Name, Caption, Description` | 每条主板记录一项，厂商与产品拼接，名称缺失按下表回退 | 没有使用统一虚拟属性，不猜测 |
| CPU | `Win32_Processor.Name, Caption, Description, NumberOfCores, NumberOfLogicalProcessors, L2CacheSize, L3CacheSize` | 每条处理器记录一项；缓存由 KiB 换算为字节再格式化 | 没有使用统一虚拟属性，不猜测 |
| 内存 | `Win32_PhysicalMemory.Manufacturer, PartNumber, Name, Caption, Description, Capacity, SMBIOSMemoryType, ConfiguredClockSpeed`；`Win32_PhysicalMemoryArray.MemoryDevices` | 每条内存设备一项；内存设备记录数为 n，全部内存阵列报告的 `MemoryDevices` 之和为 m，展示“系统报告已使用插槽 n / m” | 不根据厂商名称或容量推断虚拟化；插槽数量仅表示系统报告 |
| 显卡 | DXGI `EnumAdapters1/GetDesc1`；DisplayConfig 适配器接口；PnP `DEVPKEY_Device_InstanceId, FriendlyName, DeviceDesc`；WMI `Win32_VideoController.Name, Caption, Description, PNPDeviceID`；内核 `D3DKMTQueryAdapterInfo(KMTQAITYPE_ADAPTERTYPE)` | 按设备实例 ID 将 WMI 名称与 DXGI 显存信息组合；详情见下节 | 内核 `IndirectDisplayDevice` 或 `Paravirtualized` 位；软件渲染器先按 DXGI `SOFTWARE` 标志排除 |
| 显示器 | `QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS)` 与 `DisplayConfigGetDeviceInfo(GET_TARGET_NAME)`；`root/WMI:WmiMonitorID`；`WmiMonitorBasicDisplayParams`，WMI 两类均筛选 `Active = TRUE`；PnP `FriendlyName/DeviceDesc` | 每条活动目标路径一项；目标接口完整实例与 WMI `InstanceName` 关联，名称和尺寸只与该实例组合 | 路径 `outputTechnology = INDIRECT_VIRTUAL`；不把有线间接显示 `INDIRECT_WIRED`、Miracast 或支持虚拟分辨率直接标成虚拟显示器 |
| 磁盘 | `root/Microsoft/Windows/Storage:MSFT_Disk.FriendlyName, Model, Manufacturer, Size, BusType` | 每条系统磁盘记录一项，单一来源，不拼接旧 `Win32_DiskDrive` 结果 | `BusType` 为 14（Virtual）、15（File Backed Virtual）或 16（Storage Spaces）时标注；这里展示系统磁盘对象，包含虚拟磁盘及存储空间逻辑磁盘 |
| 电池 | `Win32_Battery.Name, Caption, Description, PNPDeviceID`；`root/WMI:BatteryStaticData.InstanceName, DeviceName, DesignedCapacity, Capabilities`；`BatteryFullChargedCapacity.InstanceName, FullChargedCapacity`；`BatteryCycleCount.InstanceName, CycleCount` | 静态、满充和循环记录按非空 `InstanceName` 关联；静态名称缺失时仅从实例匹配的基础电池补名称；没有静态数据时展示全部基础电池记录；没有电池时省略该行 | 没有统一虚拟属性，不猜测 |
| 声卡 | `CM_Get_Device_Interface_List_SizeW/ListW(KSCATEGORY_AUDIO, ALL_DEVICES)`；接口 `DEVPKEY_Device_InstanceId, DEVPKEY_DeviceInterface_FriendlyName`；`CM_Locate_DevNodeW(NORMAL)`；`CM_Get_Parent`；节点 `DEVPKEY_Device_ClassGuid, FriendlyName, DeviceDesc, BusTypeGuid` | 同一设备的 wave、capture、topology 接口归到其 PnP 节点；连续 `GUID_DEVCLASS_MEDIA` 父子节点归到最上层音频适配器；按该节点身份组合，不按名称或公共 PCI/USB 父节点合并 | 读取归并后适配器自身的总线属性，`GUID_BUS_TYPE_SW_DEVICE` 时标注；其他或缺失总线仍未确认，不用端点的软件设备属性、ROOT/SWD 前缀或名称猜测 |
| 网卡 | `root/StandardCimv2:MSFT_NetAdapter.InterfaceDescription, Name, Virtual`；筛选 `Hidden = FALSE` | 每条正常可见网络接口一项；不拼接旧 `Win32_NetworkAdapter`，不要求接口已连接或已启用 | 严格使用 `Virtual = TRUE`；不使用旧 `PhysicalAdapter`、接口名称或 TUN/TAP 关键词推断 |

## 名称回退

下列顺序中的名称均缺失时使用“未知设备”，不删除设备记录。

| 类别 | 名称来源优先顺序 |
| --- | --- |
| 主板 | `Manufacturer + Product` → `Name` → `Caption` → `Description` |
| CPU | `Name` → `Caption` → `Description` |
| 内存 | `Manufacturer` → `PartNumber` → `Name` → `Caption` → `Description` |
| 显卡 | WMI 的 `Name/Caption/Description` → 同实例 PnP `FriendlyName/DeviceDesc`；DXGI 的 `Description` → 同实例 PnP 名称。按实例组合时保留先取得的可用名称，WMI 名称缺失时允许 DXGI 补充 |
| 显示器 | 对应 WMI `UserFriendlyName` → 目标路径友好名称 → 同实例 PnP `FriendlyName/DeviceDesc` → 接口友好名称 |
| 磁盘 | `FriendlyName` → `Model` → `Manufacturer` |
| 电池 | 静态 `DeviceName` → 实例匹配的基础电池 `Name/Caption/Description` → 同实例 PnP 名称；无静态数据时直接使用基础记录的同一回退顺序 |
| 声卡 | 适配器 PnP `FriendlyName` → `DeviceDesc` → 最近的有名称的音频功能节点（该节点的 PnP 名称，或枚举接口友好名称） |
| 网卡 | `InterfaceDescription` → `Name` |

## 声卡枚举与组合

1. 使用原生 `KSCATEGORY_AUDIO` 设备接口取代 `Win32_SoundDevice` 单一来源，不直接把全部 MEDIA 类设备或每个扬声器／麦克风端点当作声卡。
2. 枚举全部已注册接口，再通过 `CM_Locate_DevNodeW(NORMAL)` 保留当前配置中的设备，包括禁用设备；忽略已移除设备遗留的注册接口。
3. 每个接口通过 `DEVPKEY_Device_InstanceId` 找到自身设备。同一实例的多个 wave、capture、topology 接口先归为一个设备。
4. 沿 `CM_Get_Parent` 读取连续 `GUID_DEVCLASS_MEDIA` 音频父子关系，在非音频类别处停止，避免把同一 PCI 控制器下的 Senary 与显示音频误合并。适配器使用当前查询中对应 PnP 实例的 `DEVINST` 身份归并。
5. 优先保留音频适配器名称；适配器无名称时使用最近的有名称的音频功能节点；全部缺失仍输出“未知设备”。接口身份或节点读取失败时保留可读接口名称或未知记录，并报告错误，不按名称合并这些未确认身份的记录。
6. 虚拟类型只取归并后的适配器自身 `BusTypeGuid`。不把真实声卡之上的 SWD 音频端点当作虚拟声卡，也不把子功能节点的软件总线属性直接提升为整张声卡的类型。
7. 本机旧查询仅返回 Senary Audio；原生接口还能发现显示音频。适配器父节点及其子功能节点最终只形成一个 `HD Audio Driver for Display Audio` 条目，与 Senary 分开保留。

这里覆盖的是注册了标准音频接口的当前配置设备；不声称能枚举驱动未安装或未注册此类接口的硬件，也不声称所有第三方驱动都会提供可确认虚拟类型的属性。

## 显卡身份与数据组合

1. `CreateDXGIFactory1/EnumAdapters1` 枚举逻辑适配器，读取描述、专用显存、共享内存上限及 `AdapterLuid`。
2. `DXGI_ADAPTER_FLAG_SOFTWARE` 表示软件渲染适配器，直接排除。Microsoft Basic Render Driver 属于这类，不将其标成独立虚拟显卡。
3. LUID 仅作为本次内核及显示 API 调用的身份，不等同于物理显卡数量，也不作为与 WMI 组合的键。
4. 通过 LUID 调用 `DisplayConfigGetDeviceInfo(GET_ADAPTER_NAME)` 获取枚举适配器自身的设备接口。
5. 通过 `CM_Get_Device_Interface_PropertyW(DEVPKEY_Device_InstanceId)` 获取接口所属设备实例 ID，与 WMI `PNPDeviceID` 不区分大小写匹配。
6. WMI 名称优先；DXGI 为同一个实例补充显存。相同实例的多个来源组合成一项，同型号但实例不同的设备分别保留。身份缺失的记录保留，不按名称合并。
7. `D3DKMTOpenAdapterFromLuid` 打开内核适配器，`KMTQAITYPE_ADAPTERTYPE` 获取类型，完成后关闭句柄。`IndirectDisplayDevice`（间接显示适配器）及 `Paravirtualized`（半虚拟化适配器）按虚拟显卡展示策略标注。此处标注的是逻辑显卡适配器；其连接的显示器是否虚拟另由显示路径属性决定。
8. 不使用 `KMTQAITYPE_PHYSICALADAPTERPNPKEY` 代替逻辑设备身份：虚拟适配器可能在该查询中返回底层硬件 GPU 的键。
9. 虚拟显卡不展示 DXGI 返回的显存数值，以免将底层 GPU 的显存误报为虚拟设备独占资源。普通适配器显示专用显存与共享上限，不将共享上限表述为当前占用。

本机曾出现两个 DXGI 适配器均名为 Intel(R) Graphics、型号和显存相同、LUID 不同。其中一个接口属于 Intel 的 PCI 设备，另一个接口属于 GameViewer 的 ROOT 显示设备。按实例关联后分别显示 Intel 与 GameViewer，而非按名称删除其中一项。

## 显示器关联与当前模式

- 枚举活动显示路径；每条路径的目标由 `adapterId + targetId` 标识。
- `GET_TARGET_NAME` 返回监视器接口路径及友好名称；完整接口实例与 WMI 的 `InstanceName` 匹配，仅去掉 WMI 实例的末尾编号后缀，不只按型号匹配。
- 名称依次取对应 WMI `UserFriendlyName`、当前目标的系统友好名称、同一 PnP 实例的 `DEVPKEY_Device_FriendlyName/DEVPKEY_Device_DeviceDesc`、接口友好名称，均缺失时显示“未知设备”。
- 宽高厘米只与同一 `InstanceName` 的 `WmiMonitorBasicDisplayParams` 组合，对角线换算为英寸。
- 桌面分辨率读取本次查询返回的源模式，主屏标记对应源模式位置 `(0, 0)`；刷新率读取目标路径的有理数刷新率，保留最多两位小数。
- 当前查询不使用 `VIRTUAL_MODE_AWARE`，因此读取普通 `modeInfoIdx`。虚拟分辨率支持与虚拟设备性质是不同属性。

## 电池计算

- 设计和满充容量有效值参与展示，零值、`u32::MAX` 无效哨兵不参与计算。
- `Capabilities` 的 `BATTERY_CAPACITY_RELATIVE` 位决定按相对单位或 Wh 展示；能力属性缺失时保留“提供程序单位”说明。
- 健康度为满充容量 / 设计容量，显示值最高为 100%。
- 循环次数的 `u32::MAX` 不展示。
- `InstanceName` 缺失时不将不同来源的第一条记录误拼为同一块电池。
- 从基础电池补名称时，仅比较 `PNPDeviceID` 与 WMI `InstanceName` 的完整身份；允许去除提供程序附加的数字后缀，不按下标或名称关联。

## 类型属性的边界

网卡直接保留 `Virtual` 的真、假或缺失。显卡的内核类型读取失败、没有 DXGI 对应项时保持未确认；类型成功读取后按相同标志判定。磁盘的总线属性缺失或 `BusType = 0`（Unknown）时保持未确认。显示器的输出技术为 `OTHER` 时保持未确认，其余按 `INDIRECT_VIRTUAL` 判定。声卡的软件总线可确认虚拟，其他总线与属性缺失均保持未确认，不据此确认物理设备。主板、CPU、内存和电池没有查询可靠虚拟类型属性，保持未确认。

“不按名称猜测”和“所有驱动都能确认虚拟类型”不能同时保证。驱动或管理类没有报告可靠属性时，当前结果只表示设备存在，未标注不等于已证明是物理硬件。尤其是第三方 ROOT 枚举的虚拟声卡，不保证提供 `GUID_BUS_TYPE_SW_DEVICE`；此时保留名称并明确将类型识别视为未确认，不回退到关键词规则。

主板、CPU、内存和电池保留实际管理类提供的信息，不从 HypervisorPresent、ROOT、厂商名称或产品字符串推断单个组件的虚拟属性，也不承诺从虚拟机内还原宿主物理硬件。

## 验证记录

自动化测试覆盖按设备实例组合显卡、同型号不同实例保留、虚拟适配器类型传播、身份缺失不按名称合并、显示器完整实例匹配、全部类别的稳定类型排序与无名称设备保留、名称属性回退、声卡接口与音频父子节点归并、相同声卡名称的不同实例保留、主屏与虚拟属性同时显示、右侧图标对齐、复制文本及系统信息页面布局与文本对比度。运行本机只读测试可检查实际 MI、DXGI、KMT、DisplayConfig、PnP 和存储/网卡提供程序结果；它与真实应用窗口验收分开记录。

2026-11-12：拆分检测器及声卡改用原生接口后，全量测试 46 项通过，1 项本机查询默认忽略；另行在具备 WMI 读取权限的环境执行本机只读查询通过，9 类共 18 条设备。声卡为 `HD Audio Driver for Display Audio` 与 `Senary Audio` 两项，显示音频的父子节点未重复计数；内存为“系统报告已使用插槽 1 / 2”。Intel 显卡位于 GameViewer 虚拟显卡之前，Realtek 与 Intel Wi-Fi 位于虚拟网络接口之前。页面无界面测试覆盖深浅主题、620/900 宽度、长文字换行、单标记与双标记的右侧对齐和复制结果。严格 Clippy、格式检查通过。本次按用户要求未进行真实应用窗口检查。

## 系统接口依据

- [D3DKMT_ADAPTERTYPE](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ns-d3dkmthk-_d3dkmt_adaptertype)
- [MSFT_NetAdapter](https://learn.microsoft.com/en-us/windows/win32/fwp/wmi/netadaptercimprov/msft-netadapter)
- [MSFT_Disk](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-disk)
- [QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)
- [DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ne-wingdi-displayconfig_video_output_technology)
- [DEVPKEY_Device_BusTypeGuid](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devpkey-device-bustypeguid)
- [Installing Device Interfaces for an Audio Adapter](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/installing-device-interfaces-for-an-audio-adapter)
- [CM_Get_Device_Interface_ListW](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_device_interface_listw)
- [CM_Locate_DevNodeW](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_locate_devnodew)
- [CM_Get_Parent](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_parent)
