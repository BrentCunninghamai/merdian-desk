import 'package:flutter/material.dart';
import 'package:flutter_hbb/desktop/widgets/merdian_attended_support.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('attended guide shows ID, Accept and end-session instructions',
      (tester) async {
    var exited = false;
    await tester.pumpWidget(MaterialApp(
      theme: merdianTheme(ThemeData()),
      home: Scaffold(
        body: SizedBox(
          width: 260,
          child: SingleChildScrollView(
            child: Column(children: [
              const MerdianBrandBadge(),
              MerdianSupportGuide(onExit: () => exited = true),
            ]),
          ),
        ),
      ),
    ));
    expect(find.text('Merdian-Desk'), findsOneWidget);
    expect(find.text('Share your ID'), findsOneWidget);
    expect(find.text('You choose Accept'), findsOneWidget);
    expect(
        find.textContaining('Nothing starts until you Accept'), findsOneWidget);
    expect(find.textContaining('Disconnect in your session window'),
        findsOneWidget);
    await tester.ensureVisible(find.text('Exit Merdian-Desk'));
    await tester.tap(find.text('Exit Merdian-Desk'));
    expect(exited, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('guide remains readable at narrow width and larger text',
      (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(MaterialApp(
      theme: merdianTheme(ThemeData.dark()),
      home: Scaffold(
        body: MediaQuery(
          data: const MediaQueryData(textScaler: TextScaler.linear(1.5)),
          child: SizedBox(
            width: 260,
            child: SingleChildScrollView(
              child: Column(children: [
                const MerdianBrandBadge(),
                MerdianSupportGuide(onExit: () {}),
              ]),
            ),
          ),
        ),
      ),
    ));
    await tester.ensureVisible(find.text('Exit Merdian-Desk'));
    expect(find.bySemanticsLabel('Merdian-Desk'), findsWidgets);
    expect(find.text('Exit Merdian-Desk'), findsOneWidget);
    expect(tester.takeException(), isNull);
    semantics.dispose();
  });

  test('light and dark themes retain high-contrast brand action colors', () {
    final light = merdianTheme(ThemeData());
    final dark = merdianTheme(ThemeData.dark());
    expect(light.colorScheme.primary, merdianTeal);
    expect(light.colorScheme.onPrimary, Colors.white);
    expect(dark.colorScheme.primary, merdianLightTeal);
    expect(dark.colorScheme.onPrimary, const Color(0xFF003C37));
    expect(light.brightness, Brightness.light);
    expect(dark.brightness, Brightness.dark);
  });
}
