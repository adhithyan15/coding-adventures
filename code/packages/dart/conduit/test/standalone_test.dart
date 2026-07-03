import 'dart:io';
import 'package:coding_adventures_conduit/conduit.dart';

Future<void> main() async {
  print('Testing NativeCallable.isolateLocal from a standalone main()');
  
  final server = Application()
    .get('/', (req) => Response.text('hello from dart'))
    .bind('127.0.0.1', 0);
  
  server.serveBackground();
  await Future.delayed(const Duration(milliseconds: 100));
  
  final port = server.localPort;
  print('Server on port $port');
  
  final client = HttpClient();
  try {
    final req = await client.get('127.0.0.1', port, '/');
    final resp = await req.close();
    final body = await resp.transform(systemEncoding.decoder).join();
    print('Response: ${resp.statusCode} - $body');
    if (body == 'hello from dart') {
      print('PASSED');
    } else {
      print('FAILED: unexpected body: $body');
      exit(1);
    }
  } finally {
    client.close();
    server.dispose();
  }
}
