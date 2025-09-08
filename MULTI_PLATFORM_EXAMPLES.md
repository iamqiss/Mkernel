# Neo Messaging Kernel - Multi-Platform Examples

## 🚀 Overview

This document provides comprehensive examples of using Neo Messaging Kernel across all supported platforms: Swift (iOS), Kotlin (Android), Flutter (cross-platform), React Native (cross-platform), and PWA (Progressive Web Apps).

## 📱 Mobile Examples

### iOS (Swift) Example

#### 1. **Basic RPC Call**

```swift
import NeoProtocol

// Define your service
struct UserService {
    static let shared = UserService()
    private let client: NeoClient
    
    private init() {
        let config = NeoClientConfig(
            serverURL: URL(string: "https://api.example.com")!,
            timeout: 30.0,
            enableTLS: true
        )
        self.client = NeoClient(config: config)
    }
    
    // Connect to server
    func connect() async throws {
        try await client.connect()
    }
    
    // Get user by ID
    func getUser(id: UInt64) async throws -> User {
        return try await client.call(
            service: "UserService",
            method: "GetUser",
            request: UserId(id: id),
            responseType: User.self
        )
    }
    
    // Create user
    func createUser(_ user: CreateUserRequest) async throws -> UserId {
        return try await client.call(
            service: "UserService",
            method: "CreateUser",
            request: user,
            responseType: UserId.self
        )
    }
    
    // Subscribe to user events
    func subscribeToUserEvents() -> AsyncThrowingStream<UserEvent, Error> {
        return client.subscribe(
            topic: "users.events",
            eventType: UserEvent.self
        )
    }
}

// Usage in your app
class UserViewController: UIViewController {
    private let userService = UserService.shared
    
    override func viewDidLoad() {
        super.viewDidLoad()
        
        Task {
            do {
                try await userService.connect()
                let user = try await userService.getUser(id: 123)
                print("User: \(user)")
            } catch {
                print("Error: \(error)")
            }
        }
    }
}
```

#### 2. **Real-time Event Streaming**

```swift
import NeoProtocol
import Combine

class UserEventManager: ObservableObject {
    @Published var users: [User] = []
    @Published var isConnected = false
    
    private let userService = UserService.shared
    private var cancellables = Set<AnyCancellable>()
    
    func startListening() {
        Task {
            do {
                try await userService.connect()
                isConnected = true
                
                // Listen to user events
                for try await event in userService.subscribeToUserEvents() {
                    await MainActor.run {
                        switch event.type {
                        case .userCreated:
                            users.append(event.user)
                        case .userUpdated:
                            if let index = users.firstIndex(where: { $0.id == event.user.id }) {
                                users[index] = event.user
                            }
                        case .userDeleted:
                            users.removeAll { $0.id == event.user.id }
                        }
                    }
                }
            } catch {
                print("Error listening to events: \(error)")
                isConnected = false
            }
        }
    }
}
```

### Android (Kotlin) Example

#### 1. **Basic RPC Call**

```kotlin
import com.neo.protocol.*
import kotlinx.coroutines.*

class UserService {
    companion object {
        val instance = UserService()
    }
    
    private val client: NeoClient
    
    init {
        val config = NeoClientConfig(
            serverUrl = "https://api.example.com",
            timeout = 30000,
            enableTLS = true
        )
        this.client = NeoClient(config)
    }
    
    suspend fun connect() {
        client.connect()
    }
    
    suspend fun getUser(id: ULong): User {
        return client.call(
            service = "UserService",
            method = "GetUser",
            request = UserId(id = id),
            responseType = User::class
        )
    }
    
    suspend fun createUser(user: CreateUserRequest): UserId {
        return client.call(
            service = "UserService",
            method = "CreateUser",
            request = user,
            responseType = UserId::class
        )
    }
    
    fun subscribeToUserEvents(): Flow<UserEvent> {
        return client.subscribe(
            topic = "users.events",
            eventType = UserEvent::class
        )
    }
}

// Usage in your Activity
class MainActivity : AppCompatActivity() {
    private val userService = UserService.instance
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        
        lifecycleScope.launch {
            try {
                userService.connect()
                val user = userService.getUser(123)
                Log.d("UserService", "User: $user")
            } catch (e: Exception) {
                Log.e("UserService", "Error: $e")
            }
        }
    }
}
```

#### 2. **Real-time Event Streaming with Flow**

```kotlin
import com.neo.protocol.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

class UserEventManager : ViewModel() {
    private val _users = MutableStateFlow<List<User>>(emptyList())
    val users: StateFlow<List<User>> = _users.asStateFlow()
    
    private val _isConnected = MutableStateFlow(false)
    val isConnected: StateFlow<Boolean> = _isConnected.asStateFlow()
    
    private val userService = UserService.instance
    
    fun startListening() {
        viewModelScope.launch {
            try {
                userService.connect()
                _isConnected.value = true
                
                userService.subscribeToUserEvents()
                    .collect { event ->
                        when (event.type) {
                            UserEventType.USER_CREATED -> {
                                _users.value = _users.value + event.user
                            }
                            UserEventType.USER_UPDATED -> {
                                _users.value = _users.value.map { user ->
                                    if (user.id == event.user.id) event.user else user
                                }
                            }
                            UserEventType.USER_DELETED -> {
                                _users.value = _users.value.filter { it.id != event.user.id }
                            }
                        }
                    }
            } catch (e: Exception) {
                Log.e("UserEventManager", "Error: $e")
                _isConnected.value = false
            }
        }
    }
}
```

### Flutter (Dart) Example

#### 1. **Basic RPC Call**

```dart
import 'package:neo_protocol/neo_protocol.dart';

class UserService {
  static final UserService _instance = UserService._internal();
  factory UserService() => _instance;
  UserService._internal();
  
  late final NeoClient _client;
  
  Future<void> initialize() async {
    final config = NeoClientConfig(
      serverUrl: 'https://api.example.com',
      timeout: Duration(seconds: 30),
      enableTLS: true,
    );
    _client = NeoClient(config);
    await _client.connect();
  }
  
  Future<User> getUser(int id) async {
    return await _client.call(
      service: 'UserService',
      method: 'GetUser',
      request: UserId(id: id),
      responseType: User,
    );
  }
  
  Future<UserId> createUser(CreateUserRequest user) async {
    return await _client.call(
      service: 'UserService',
      method: 'CreateUser',
      request: user,
      responseType: UserId,
    );
  }
  
  Stream<UserEvent> subscribeToUserEvents() {
    return _client.subscribe(
      topic: 'users.events',
      eventType: UserEvent,
    );
  }
}

// Usage in your widget
class UserScreen extends StatefulWidget {
  @override
  _UserScreenState createState() => _UserScreenState();
}

class _UserScreenState extends State<UserScreen> {
  final UserService _userService = UserService();
  User? _user;
  bool _isLoading = false;
  
  @override
  void initState() {
    super.initState();
    _loadUser();
  }
  
  Future<void> _loadUser() async {
    setState(() => _isLoading = true);
    
    try {
      await _userService.initialize();
      final user = await _userService.getUser(123);
      setState(() => _user = user);
    } catch (e) {
      print('Error: $e');
    } finally {
      setState(() => _isLoading = false);
    }
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('User')),
      body: _isLoading
          ? Center(child: CircularProgressIndicator())
          : _user != null
              ? UserWidget(user: _user!)
              : Center(child: Text('No user data')),
    );
  }
}
```

#### 2. **Real-time Event Streaming with StreamBuilder**

```dart
import 'package:neo_protocol/neo_protocol.dart';
import 'package:flutter/material.dart';

class UserEventScreen extends StatefulWidget {
  @override
  _UserEventScreenState createState() => _UserEventScreenState();
}

class _UserEventScreenState extends State<UserEventScreen> {
  final UserService _userService = UserService();
  late Stream<UserEvent> _eventStream;
  
  @override
  void initState() {
    super.initState();
    _initializeService();
  }
  
  Future<void> _initializeService() async {
    await _userService.initialize();
    _eventStream = _userService.subscribeToUserEvents();
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('User Events')),
      body: StreamBuilder<UserEvent>(
        stream: _eventStream,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Center(child: Text('Error: ${snapshot.error}'));
          }
          
          if (snapshot.hasData) {
            final event = snapshot.data!;
            return ListView(
              children: [
                ListTile(
                  title: Text('Event: ${event.type}'),
                  subtitle: Text('User: ${event.user.name}'),
                  trailing: Text(event.timestamp.toString()),
                ),
              ],
            );
          }
          
          return Center(child: CircularProgressIndicator());
        },
      ),
    );
  }
}
```

## 🌐 Web Examples

### React Native Example

#### 1. **Basic RPC Call**

```typescript
import { NeoClient, NeoClientConfig } from '@neo/protocol';

class UserService {
  private static instance: UserService;
  private client: NeoClient;
  
  private constructor() {
    const config = new NeoClientConfig({
      serverUrl: 'https://api.example.com',
      timeout: 30000,
      enableTLS: true,
    });
    this.client = new NeoClient(config);
  }
  
  static getInstance(): UserService {
    if (!UserService.instance) {
      UserService.instance = new UserService();
    }
    return UserService.instance;
  }
  
  async connect(): Promise<void> {
    await this.client.connect();
  }
  
  async getUser(id: number): Promise<User> {
    return await this.client.call(
      'UserService',
      'GetUser',
      new UserId({ id }),
      User
    );
  }
  
  async createUser(user: CreateUserRequest): Promise<UserId> {
    return await this.client.call(
      'UserService',
      'CreateUser',
      user,
      UserId
    );
  }
  
  subscribeToUserEvents(): Observable<UserEvent> {
    return this.client.subscribe(
      'users.events',
      UserEvent
    );
  }
}

// Usage in your React Native component
import React, { useEffect, useState } from 'react';
import { View, Text, Button, FlatList } from 'react-native';

const UserScreen: React.FC = () => {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const userService = UserService.getInstance();
  
  useEffect(() => {
    loadUser();
  }, []);
  
  const loadUser = async () => {
    setIsLoading(true);
    try {
      await userService.connect();
      const userData = await userService.getUser(123);
      setUser(userData);
    } catch (error) {
      console.error('Error loading user:', error);
    } finally {
      setIsLoading(false);
    }
  };
  
  return (
    <View style={{ flex: 1, padding: 20 }}>
      {isLoading ? (
        <Text>Loading...</Text>
      ) : user ? (
        <View>
          <Text>Name: {user.name}</Text>
          <Text>Email: {user.email}</Text>
          <Text>ID: {user.id}</Text>
        </View>
      ) : (
        <Text>No user data</Text>
      )}
    </View>
  );
};

export default UserScreen;
```

#### 2. **Real-time Event Streaming with Hooks**

```typescript
import React, { useEffect, useState } from 'react';
import { View, Text, FlatList } from 'react-native';
import { Observable } from 'rxjs';

const UserEventScreen: React.FC = () => {
  const [events, setEvents] = useState<UserEvent[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const userService = UserService.getInstance();
  
  useEffect(() => {
    const initializeService = async () => {
      try {
        await userService.connect();
        setIsConnected(true);
        
        // Subscribe to events
        userService.subscribeToUserEvents().subscribe({
          next: (event) => {
            setEvents(prev => [...prev, event]);
          },
          error: (error) => {
            console.error('Event stream error:', error);
            setIsConnected(false);
          }
        });
      } catch (error) {
        console.error('Error initializing service:', error);
      }
    };
    
    initializeService();
  }, []);
  
  const renderEvent = ({ item }: { item: UserEvent }) => (
    <View style={{ padding: 10, borderBottomWidth: 1 }}>
      <Text>Type: {item.type}</Text>
      <Text>User: {item.user.name}</Text>
      <Text>Time: {new Date(item.timestamp).toLocaleString()}</Text>
    </View>
  );
  
  return (
    <View style={{ flex: 1 }}>
      <Text>Status: {isConnected ? 'Connected' : 'Disconnected'}</Text>
      <FlatList
        data={events}
        renderItem={renderEvent}
        keyExtractor={(item, index) => index.toString()}
      />
    </View>
  );
};

export default UserEventScreen;
```

### PWA Example

#### 1. **Basic RPC Call with Offline Support**

```typescript
import { NeoPWA, NeoClientConfig } from '@neo/pwa';

class UserService {
  private static instance: UserService;
  private client: NeoPWA;
  
  private constructor() {
    const config = new NeoClientConfig({
      serverUrl: 'https://api.example.com',
      timeout: 30000,
      enableTLS: true,
      enableOfflineSupport: true,
      enableBackgroundSync: true,
    });
    this.client = new NeoPWA(config);
  }
  
  static getInstance(): UserService {
    if (!UserService.instance) {
      UserService.instance = new UserService();
    }
    return UserService.instance;
  }
  
  async connect(): Promise<void> {
    await this.client.connect();
  }
  
  async getUser(id: number, useCache: boolean = true): Promise<User> {
    return await this.client.call(
      'UserService',
      'GetUser',
      new UserId({ id }),
      User,
      { useCache }
    );
  }
  
  async createUser(user: CreateUserRequest): Promise<UserId> {
    return await this.client.call(
      'UserService',
      'CreateUser',
      user,
      UserId
    );
  }
  
  subscribeToUserEvents(): Observable<UserEvent> {
    return this.client.subscribe(
      'users.events',
      UserEvent
    );
  }
  
  async syncOfflineOperations(): Promise<void> {
    await this.client.syncOfflineOperations();
  }
}

// Usage in your PWA
class UserApp {
  private userService = UserService.getInstance();
  
  async initialize() {
    // Register service worker
    if ('serviceWorker' in navigator) {
      await navigator.serviceWorker.register('/service-worker.js');
    }
    
    // Connect to server
    await this.userService.connect();
    
    // Set up offline sync
    await this.userService.syncOfflineOperations();
  }
  
  async loadUser(id: number) {
    try {
      const user = await this.userService.getUser(id);
      this.displayUser(user);
    } catch (error) {
      console.error('Error loading user:', error);
      // Handle offline case
      this.showOfflineMessage();
    }
  }
  
  private displayUser(user: User) {
    // Update UI with user data
    document.getElementById('user-name')!.textContent = user.name;
    document.getElementById('user-email')!.textContent = user.email;
  }
  
  private showOfflineMessage() {
    // Show offline indicator
    document.getElementById('offline-indicator')!.style.display = 'block';
  }
}

// Initialize app
const app = new UserApp();
app.initialize();
```

#### 2. **Service Worker for Offline Support**

```javascript
// service-worker.js
import { precacheAndRoute } from 'workbox-precaching';
import { registerRoute } from 'workbox-routing';
import { StaleWhileRevalidate, NetworkFirst } from 'workbox-strategies';
import { BackgroundSyncPlugin } from 'workbox-background-sync';

// Precache static assets
precacheAndRoute(self.__WB_MANIFEST);

// Cache API calls
registerRoute(
  ({ url }) => url.pathname.startsWith('/api/'),
  new NetworkFirst({
    cacheName: 'api-cache',
    plugins: [
      new BackgroundSyncPlugin('api-queue', {
        maxRetentionTime: 24 * 60, // 24 hours
      }),
    ],
  })
);

// Cache user data
registerRoute(
  ({ url }) => url.pathname.startsWith('/api/users/'),
  new StaleWhileRevalidate({
    cacheName: 'user-cache',
  })
);

// Handle offline requests
self.addEventListener('fetch', (event) => {
  if (event.request.method === 'POST' && event.request.url.includes('/api/')) {
    event.respondWith(
      fetch(event.request).catch(() => {
        // Queue for background sync
        return new Response('Offline', { status: 503 });
      })
    );
  }
});
```

## 🔧 Advanced Examples

### 1. **Error Handling and Retry Logic**

```swift
// Swift example
class ResilientUserService {
    private let client: NeoClient
    private let maxRetries: Int = 3
    private let retryDelay: TimeInterval = 1.0
    
    init(config: NeoClientConfig) {
        self.client = NeoClient(config: config)
    }
    
    func getUserWithRetry(id: UInt64) async throws -> User {
        var lastError: Error?
        
        for attempt in 1...maxRetries {
            do {
                return try await client.call(
                    service: "UserService",
                    method: "GetUser",
                    request: UserId(id: id),
                    responseType: User.self
                )
            } catch {
                lastError = error
                if attempt < maxRetries {
                    try await Task.sleep(nanoseconds: UInt64(retryDelay * Double(attempt) * 1_000_000_000))
                }
            }
        }
        
        throw lastError ?? NeoError.unknown("Max retries exceeded")
    }
}
```

### 2. **Connection Management**

```kotlin
// Kotlin example
class ConnectionManager {
    private val client: NeoClient
    private var isConnected = false
    private val reconnectDelay = 5000L
    
    init {
        val config = NeoClientConfig(
            serverUrl = "https://api.example.com",
            timeout = 30000,
            enableTLS = true
        )
        this.client = NeoClient(config)
    }
    
    suspend fun ensureConnected() {
        if (!isConnected) {
            try {
                client.connect()
                isConnected = true
            } catch (e: Exception) {
                isConnected = false
                delay(reconnectDelay)
                ensureConnected()
            }
        }
    }
    
    fun startHeartbeat() {
        GlobalScope.launch {
            while (true) {
                try {
                    ensureConnected()
                    delay(30000) // Heartbeat every 30 seconds
                } catch (e: Exception) {
                    isConnected = false
                    delay(reconnectDelay)
                }
            }
        }
    }
}
```

### 3. **Custom Authentication**

```dart
// Flutter example
class AuthenticatedUserService {
  late final NeoClient _client;
  String? _authToken;
  
  Future<void> initialize() async {
    final config = NeoClientConfig(
      serverUrl: 'https://api.example.com',
      timeout: Duration(seconds: 30),
      enableTLS: true,
      headers: {
        'Authorization': 'Bearer $_authToken',
      },
    );
    _client = NeoClient(config);
    await _client.connect();
  }
  
  Future<void> authenticate(String username, String password) async {
    // Authenticate with your auth service
    final authResponse = await _authenticateWithServer(username, password);
    _authToken = authResponse.token;
    
    // Reinitialize client with new token
    await initialize();
  }
  
  Future<AuthResponse> _authenticateWithServer(String username, String password) async {
    // Implement your authentication logic
    throw UnimplementedError();
  }
}
```

## 📊 Performance Examples

### 1. **Batch Operations**

```typescript
// TypeScript example
class BatchUserService {
  private client: NeoClient;
  private batchQueue: Array<() => Promise<any>> = [];
  private batchSize = 10;
  private batchTimeout = 1000; // 1 second
  
  constructor(client: NeoClient) {
    this.client = client;
    this.startBatchProcessor();
  }
  
  async getUser(id: number): Promise<User> {
    return new Promise((resolve, reject) => {
      this.batchQueue.push(async () => {
        try {
          const user = await this.client.call(
            'UserService',
            'GetUser',
            new UserId({ id }),
            User
          );
          resolve(user);
        } catch (error) {
          reject(error);
        }
      });
    });
  }
  
  private startBatchProcessor() {
    setInterval(async () => {
      if (this.batchQueue.length > 0) {
        const batch = this.batchQueue.splice(0, this.batchSize);
        await Promise.all(batch.map(operation => operation()));
      }
    }, this.batchTimeout);
  }
}
```

### 2. **Caching and Optimization**

```swift
// Swift example
class CachedUserService {
    private let client: NeoClient
    private let cache = NSCache<NSString, User>()
    private let cacheTimeout: TimeInterval = 300 // 5 minutes
    
    init(config: NeoClientConfig) {
        self.client = NeoClient(config: config)
        self.cache.countLimit = 100
    }
    
    func getUser(id: UInt64) async throws -> User {
        let cacheKey = NSString(string: "user_\(id)")
        
        // Check cache first
        if let cachedUser = cache.object(forKey: cacheKey) {
            return cachedUser
        }
        
        // Fetch from server
        let user = try await client.call(
            service: "UserService",
            method: "GetUser",
            request: UserId(id: id),
            responseType: User.self
        )
        
        // Cache the result
        cache.setObject(user, forKey: cacheKey)
        
        return user
    }
}
```

## 🧪 Testing Examples

### 1. **Unit Testing**

```swift
// Swift test example
import XCTest
@testable import NeoProtocol

class UserServiceTests: XCTestCase {
    var userService: UserService!
    var mockClient: MockNeoClient!
    
    override func setUp() {
        super.setUp()
        mockClient = MockNeoClient()
        userService = UserService(client: mockClient)
    }
    
    func testGetUser() async throws {
        // Given
        let expectedUser = User(id: 123, name: "John Doe", email: "john@example.com")
        mockClient.mockResponse = expectedUser
        
        // When
        let user = try await userService.getUser(id: 123)
        
        // Then
        XCTAssertEqual(user.id, expectedUser.id)
        XCTAssertEqual(user.name, expectedUser.name)
        XCTAssertEqual(user.email, expectedUser.email)
    }
}
```

### 2. **Integration Testing**

```kotlin
// Kotlin test example
class UserServiceIntegrationTest {
    private lateinit var userService: UserService
    
    @BeforeEach
    fun setUp() {
        val config = NeoClientConfig(
            serverUrl = "https://test-api.example.com",
            timeout = 5000,
            enableTLS = false
        )
        userService = UserService(config)
    }
    
    @Test
    fun `test get user integration`() = runTest {
        // Given
        userService.connect()
        
        // When
        val user = userService.getUser(123)
        
        // Then
        assertThat(user.id).isEqualTo(123)
        assertThat(user.name).isNotEmpty()
    }
}
```

## 🚀 Migration Examples

### 1. **REST to Neo Migration**

```bash
# Migrate REST API to Neo
neo migrate rest \
  --input openapi.yaml \
  --output user-service.neo \
  --base-url https://api.example.com \
  --auth "Bearer ${API_TOKEN}"

# Generate client SDKs
neo generate clients \
  --service user-service.neo \
  --output ./clients \
  --platforms swift,kotlin,flutter
```

### 2. **gRPC to Neo Migration**

```bash
# Migrate gRPC service to Neo
neo migrate grpc \
  --input user.proto \
  --output user-service.neo \
  --server-url grpc.example.com:443 \
  --tls

# Generate client SDKs
neo generate clients \
  --service user-service.neo \
  --output ./clients \
  --platforms all
```

### 3. **GraphQL to Neo Migration**

```bash
# Migrate GraphQL schema to Neo
neo migrate graphql \
  --input schema.graphql \
  --output user-service.neo \
  --endpoint https://api.example.com/graphql \
  --auth "Bearer ${GRAPHQL_TOKEN}"

# Generate client SDKs
neo generate clients \
  --service user-service.neo \
  --output ./clients \
  --platforms react-native,pwa,web
```

## 📚 Best Practices

### 1. **Error Handling**
- Always handle network errors gracefully
- Implement retry logic with exponential backoff
- Provide meaningful error messages to users
- Log errors for debugging

### 2. **Performance**
- Use connection pooling where possible
- Implement caching for frequently accessed data
- Batch operations when possible
- Monitor and optimize memory usage

### 3. **Security**
- Always use TLS in production
- Implement proper authentication
- Validate all inputs
- Use certificate pinning on mobile

### 4. **Testing**
- Write unit tests for all service methods
- Implement integration tests
- Test error scenarios
- Use mocking for external dependencies

### 5. **Monitoring**
- Log important events
- Monitor connection status
- Track performance metrics
- Set up alerts for failures

---

**Built with ⚡ by [Neo Qiss](https://github.com/iamqiss) in Rust 🦀**

*These examples demonstrate the power and flexibility of Neo Messaging Kernel across all platforms, providing developers with a unified API that works everywhere.*