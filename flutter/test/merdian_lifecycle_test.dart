import 'dart:async';

import 'package:flutter_hbb/desktop/merdian_lifecycle.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Exit waits for owned-session cleanup then terminates once', () async {
    final cleanup = Completer<void>();
    final events = <String>[];
    final closing = closeMerdianDesk(
      closeOwnedSessions: () {
        events.add('cleanup');
        return cleanup.future;
      },
      terminate: (code) => events.add('exit:$code'),
    );
    await Future<void>.delayed(Duration.zero);
    expect(events, ['cleanup']);
    cleanup.complete();
    await closing;
    expect(events, ['cleanup', 'exit:0']);
  });

  test('a cleanup error still terminates rather than leaving a hidden host',
      () async {
    final events = <String>[];
    await expectLater(
      closeMerdianDesk(
        closeOwnedSessions: () async => throw StateError('cleanup failed'),
        terminate: (code) => events.add('exit:$code'),
      ),
      throwsStateError,
    );
    expect(events, ['exit:0']);
  });
}
