import 'package:json_annotation/json_annotation.dart';

/// Represents a Neo protocol message
abstract class NeoMessage {
  const NeoMessage();
}

/// Represents a Neo protocol request
abstract class NeoRequest<T extends NeoMessage> extends NeoMessage {
  const NeoRequest();
}

/// Represents a Neo protocol response
abstract class NeoResponse<T extends NeoMessage> extends NeoMessage {
  const NeoResponse();
}

/// Represents a Neo protocol event
abstract class NeoEvent extends NeoMessage {
  const NeoEvent();
  
  /// The event type identifier
  static String get eventType => throw UnimplementedError('eventType must be implemented');
}

/// Base class for all Neo message types
@JsonSerializable()
class BaseNeoMessage extends NeoMessage {
  const BaseNeoMessage();
  
  factory BaseNeoMessage.fromJson(Map<String, dynamic> json) => 
      _$BaseNeoMessageFromJson(json);
  
  Map<String, dynamic> toJson() => _$BaseNeoMessageToJson(this);
}

/// Timestamp utility for Neo messages
@JsonSerializable()
class NeoTimestamp {
  final int seconds;
  final int nanoseconds;
  
  const NeoTimestamp({
    required this.seconds,
    required this.nanoseconds,
  });
  
  factory NeoTimestamp.now() {
    final now = DateTime.now();
    final milliseconds = now.millisecondsSinceEpoch;
    return NeoTimestamp(
      seconds: (milliseconds / 1000).floor(),
      nanoseconds: (milliseconds % 1000) * 1000000,
    );
  }
  
  factory NeoTimestamp.fromDateTime(DateTime dateTime) {
    final milliseconds = dateTime.millisecondsSinceEpoch;
    return NeoTimestamp(
      seconds: (milliseconds / 1000).floor(),
      nanoseconds: (milliseconds % 1000) * 1000000,
    );
  }
  
  DateTime toDateTime() {
    return DateTime.fromMillisecondsSinceEpoch(
      seconds * 1000 + (nanoseconds / 1000000).floor(),
    );
  }
  
  factory NeoTimestamp.fromJson(Map<String, dynamic> json) => 
      _$NeoTimestampFromJson(json);
  
  Map<String, dynamic> toJson() => _$NeoTimestampToJson(this);
  
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NeoTimestamp &&
          runtimeType == other.runtimeType &&
          seconds == other.seconds &&
          nanoseconds == other.nanoseconds;
  
  @override
  int get hashCode => seconds.hashCode ^ nanoseconds.hashCode;
  
  @override
  String toString() => 'NeoTimestamp(seconds: $seconds, nanoseconds: $nanoseconds)';
}

/// Binary data utility for Neo messages
@JsonSerializable()
class NeoBinary {
  final List<int> data;
  
  const NeoBinary(this.data);
  
  factory NeoBinary.fromBytes(List<int> bytes) => NeoBinary(bytes);
  
  factory NeoBinary.fromString(String string) => 
      NeoBinary(string.codeUnits);
  
  factory NeoBinary.fromBase64(String base64) => 
      NeoBinary(base64Decode(base64));
  
  List<int> get bytes => List.unmodifiable(data);
  
  String get string => String.fromCharCodes(data);
  
  String get base64 => base64Encode(data);
  
  factory NeoBinary.fromJson(Map<String, dynamic> json) => 
      _$NeoBinaryFromJson(json);
  
  Map<String, dynamic> toJson() => _$NeoBinaryToJson(this);
  
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NeoBinary &&
          runtimeType == other.runtimeType &&
          _listEquals(data, other.data);
  
  @override
  int get hashCode => data.hashCode;
  
  @override
  String toString() => 'NeoBinary(length: ${data.length})';
}

bool _listEquals<T>(List<T> a, List<T> b) {
  if (a.length != b.length) return false;
  for (int i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

// Generated code
NeoBinary _$NeoBinaryFromJson(Map<String, dynamic> json) => NeoBinary(
  (json['data'] as List<dynamic>).cast<int>(),
);

Map<String, dynamic> _$NeoBinaryToJson(NeoBinary instance) => <String, dynamic>{
  'data': instance.data,
};

NeoTimestamp _$NeoTimestampFromJson(Map<String, dynamic> json) => NeoTimestamp(
  seconds: json['seconds'] as int,
  nanoseconds: json['nanoseconds'] as int,
);

Map<String, dynamic> _$NeoTimestampToJson(NeoTimestamp instance) => <String, dynamic>{
  'seconds': instance.seconds,
  'nanoseconds': instance.nanoseconds,
};

BaseNeoMessage _$BaseNeoMessageFromJson(Map<String, dynamic> json) => BaseNeoMessage();

Map<String, dynamic> _$BaseNeoMessageToJson(BaseNeoMessage instance) => <String, dynamic>{};