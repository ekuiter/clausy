import de.ovgu.featureide.fm.core.base.IFeatureModel;
import de.ovgu.featureide.fm.core.base.impl.FMFormatManager;
import de.ovgu.featureide.fm.core.init.FMCoreLibrary;
import de.ovgu.featureide.fm.core.init.LibraryManager;
import de.ovgu.featureide.fm.core.io.IFeatureModelFormat;
import de.ovgu.featureide.fm.core.io.dimacs.DIMACSFormat;
import de.ovgu.featureide.fm.core.io.manager.FeatureModelIO;
import de.ovgu.featureide.fm.core.io.manager.FeatureModelManager;
import de.ovgu.featureide.fm.core.io.uvl.UVLFeatureModelFormat;
import de.ovgu.featureide.fm.core.io.xml.XmlFeatureModelFormat;
import de.ovgu.featureide.fm.core.job.LongRunningMethod;
import de.ovgu.featureide.fm.core.job.LongRunningWrapper;
import de.ovgu.featureide.fm.core.job.SliceFeatureModel;
import de.ovgu.featureide.fm.core.analysis.cnf.manipulator.remove.CNFSlicer;
import de.ovgu.featureide.fm.core.analysis.cnf.formula.FeatureModelFormula;
import de.ovgu.featureide.fm.core.analysis.cnf.formula.CNFCreator;
import de.ovgu.featureide.fm.core.analysis.cnf.CNF;
import de.ovgu.featureide.fm.core.base.FeatureUtils;
import de.ovgu.featureide.fm.core.io.dimacs.DimacsWriter;

import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.ArrayList;
import java.util.Collection;
import java.util.HashSet;
import java.util.Scanner;
import java.util.stream.Collectors;

public class Main {
    public static void main(String[] args) {
        if (args.length > 3)
            throw new RuntimeException("usage: java -jar io.jar [file|-] [uvl|xml|model|cnf|dimacs|sat] [feature,...]");

        LibraryManager.registerLibrary(FMCoreLibrary.getInstance());
        FMFormatManager.getInstance().addExtension(new ModelFormat());
        FMFormatManager.getInstance().addExtension(new SatFormat());

        IFeatureModel featureModel;
        if (args.length > 0 && !args[0].startsWith("-")) {
            Path inputPath = Paths.get(args[0]);
            featureModel = FeatureModelManager.load(inputPath);
        } else {
            StringBuilder sb = new StringBuilder();
            Scanner sc = new Scanner(System.in);
            while (sc.hasNextLine()) {
                sb.append(sc.nextLine());
                sb.append('\n');
            }
            featureModel = FeatureModelIO.getInstance()
                    .loadFromSource(sb, Paths.get(args.length > 0 ? args[0].replace("cnf", "dimacs") : "-.uvl"));
        }
        if (featureModel == null)
            throw new RuntimeException("failed to load feature model");

        if (args.length == 3) {
            if (args[1].equals("dimacs")) {
                Collection<String> features = Arrays.stream(args[2].split(","))
                        .filter(s -> !s.trim().isEmpty())
                        .collect(Collectors.toSet());
                if (!features.isEmpty()) {
                    ArrayList<String> removeFeatures = new ArrayList<>(FeatureUtils.getFeatureNames(featureModel));
                    removeFeatures.removeAll(features);
                    FeatureModelFormula formula = FeatureModelManager.getInstance(featureModel).getVariableFormula();
                    final CNFSlicer slicer = new CNFSlicer(formula.getElement(new CNFCreator()), removeFeatures);
                    CNF cnf = LongRunningWrapper.runMethod(slicer);
                    String output = new DimacsWriter(cnf).write();
                    System.out.print(output);
                    return;
                }
            } else {
                Collection<String> features = Arrays.stream(args[2].split(","))
                        .filter(s -> !s.trim().isEmpty())
                        .collect(Collectors.toSet());
                if (!features.isEmpty()) {
                    final LongRunningMethod<IFeatureModel> method = new SliceFeatureModel(featureModel, features, true, false);
                    featureModel = LongRunningWrapper.runMethod(method);
                    if (featureModel.getStructure().getRoot().getChildren().size() == 1) {
                        featureModel.getStructure().replaceRoot(featureModel.getStructure().getRoot().removeLastChild());
                    }
                }
            }
        }

        IFeatureModelFormat format = new SatFormat();
        if (args.length >= 2) {
            String formatString = args[1];
            switch (formatString) {
                case "uvl":
                    format = new UVLFeatureModelFormat();
                    break;
                case "xml":
                    format = new XmlFeatureModelFormat();
                    break;
                case "model":
                    format = new ModelFormat();
                    break;
                case "cnf":
                case "dimacs":
                    format = new DIMACSFormat();
                    break;
                case "sat":
                    format = new SatFormat();
                    break;
                default:
                    throw new RuntimeException("invalid format");
            }
        }

        String output = format.getInstance().write(featureModel);
        System.out.print(output);
    }
}
